#![deny(clippy::pedantic)]

use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::mem::replace;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use colored::{ColoredString, Colorize};
use glob::{GlobError, MatchOptions, Pattern, PatternError};

const SUPER_DIR: char = '\u{2502}';
const PARENT_NTH: char = '\u{251c}';
const PARENT_LAST: char = '\u{2514}';
const INDENT: &str = "\u{2500}\u{2500} ";

#[derive(Clone, Copy, Default)]
pub struct SearchOpts<'a> {
    pub dirs_only: bool,
    pub follow_symlinks: bool,
    pub show_hidden: bool,
    pub stay_on_fs: bool,
    pub max_depth: Option<usize>,
    pub use_gitignores: bool,
    pub vcs_whitelist_patterns: &'a [Pattern],
    pub vcs_blacklist_patterns: &'a [Pattern],
    pub positive_patterns: &'a [Pattern],
    pub negative_patterns: &'a [Pattern],
    pub case_insensitive_match: bool,
}

#[derive(Clone, Copy, Default)]
pub struct FormatOpts {
    pub colorize: bool,
    pub full_paths: bool,
    pub indent: bool,
    pub quote_names: bool,
}

#[derive(Clone)]
enum FType {
    Dir,
    Exe,
    File,
    Link(PathBuf),
}

impl FType {
    #[cfg(unix)]
    fn is_exec(path: &PathBuf) -> Self {
        if path.metadata().unwrap().permissions().mode() % 2 == 1 {
            FType::Exe
        } else {
            FType::File
        }
    }

    #[cfg(not(unix))]
    fn is_exec(path: &PathBuf) -> Self {
        FType::File
    }
}

#[derive(Clone)]
pub struct Dir {
    path: PathBuf,
    ftype: FType,
    read_only: bool,
    contents: Vec<Dir>,
    nest: Vec<bool>,
    format: FormatOpts,
}

impl Default for Dir {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            ftype: FType::File,
            read_only: false,
            contents: Vec::new(),
            nest: Vec::new(),
            format: FormatOpts::default(),
        }
    }
}

impl Dir {
    pub fn from(obj: &PathBuf, cfg: SearchOpts) -> Option<Self> {
        let match_opts = MatchOptions {
            case_sensitive: !cfg.case_insensitive_match,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        };

        if !cfg.show_hidden
            && Pattern::new("./.*")
                .unwrap()
                .matches_path_with(obj, &match_opts)
        {
            return None;
        }

        for pat in cfg
            .negative_patterns
            .iter()
            .chain(cfg.vcs_blacklist_patterns.iter())
        {
            if pat.matches_path_with(obj, &match_opts)
                && cfg
                    .vcs_whitelist_patterns
                    .iter()
                    .find(|p| p.matches_path_with(obj, &match_opts))
                    .is_none()
            {
                return None;
            }
        }

        let link_contents = obj
            .read_link()
            .map(|e| e.canonicalize().unwrap_or(e).starts_with(obj));
        let should_follow_link = cfg.follow_symlinks
            && (!cfg.stay_on_fs || (link_contents.is_ok() && *link_contents.as_ref().unwrap()));

        let (should_recur, max_depth) = match cfg.max_depth {
            Some(n) if n == 0 => (false, None),
            Some(n) => (true, Some(n - 1)),
            None => (true, None),
        };

        if obj.is_dir() && should_recur {
            if should_follow_link || link_contents.is_err() {
                let ignore_list = if cfg.use_gitignores {
                    VcsIgnore::in_dir_or_default(obj)
                } else {
                    VcsIgnore::default()
                }
                .compose(cfg.vcs_blacklist_patterns, cfg.vcs_whitelist_patterns);

                let contents = fs::read_dir(obj)
                    .unwrap()
                    .map(Result::unwrap)
                    .filter(|e| !cfg.dirs_only || e.metadata().unwrap().is_dir())
                    .filter_map(|e| {
                        Self::from(
                            &e.path(),
                            SearchOpts {
                                max_depth,
                                vcs_blacklist_patterns: &ignore_list.black,
                                vcs_whitelist_patterns: &ignore_list.white,
                                ..cfg
                            },
                        )
                    })
                    .collect::<Vec<_>>();

                Some(Self {
                    path: obj.to_owned(),
                    ftype: FType::Dir,
                    read_only: obj.metadata().unwrap().permissions().readonly(),
                    contents,
                    ..Default::default()
                })
            } else {
                Some(Self {
                    path: obj.to_owned(),
                    ftype: FType::Link(obj.read_link().unwrap()),
                    read_only: obj.metadata().unwrap().permissions().readonly(),
                    ..Default::default()
                })
            }
        } else {
            let mut should_stay = cfg.positive_patterns.is_empty();

            for pat in cfg.positive_patterns {
                if pat.matches_path_with(obj, &match_opts) {
                    should_stay = true;
                    break;
                }
            }

            if should_stay {
                Some(Self {
                    path: obj.to_owned(),
                    ftype: FType::is_exec(obj),
                    read_only: obj.metadata().unwrap().permissions().readonly(),
                    ..Default::default()
                })
            } else {
                None
            }
        }
    }

    fn with_nest_level(self, nest: Vec<bool>) -> Self {
        Self { nest, ..self }
    }

    pub fn with_format(self, format: FormatOpts) -> Self {
        Self { format, ..self }
    }

    fn format_name(&self) -> ColoredString {
        let stringified = if self.format.full_paths {
            self.path.to_str()
        } else {
            self.path
                .file_name()
                .unwrap_or_else(|| OsStr::new("."))
                .to_str()
        }
        .unwrap();

        let mut owned = if self.format.quote_names {
            format!("\"{}\"", stringified)
        } else {
            stringified.to_string()
        }.normal();

        if self.format.colorize {
            if self.read_only {
                owned = owned.on_red();
            }

            owned = match &self.ftype {
                FType::Dir => owned.blue(),
                FType::Exe => owned.green().bold(),
                FType::Link(loc) => format!("{} -> {:?}", owned.cyan().bold(), loc).normal(),
                FType::File => owned,
            };
        }

        owned
    }

    pub fn sort_children(&mut self) {
        self.contents.sort_unstable_by_key(|v| v.path.clone());
        self.contents.iter_mut().for_each(|c| c.sort_children());
    }

    fn has_nested_children(&self) -> bool {
        if let FType::Dir = self.ftype {
            !self.contents.is_empty()
                && self
                    .contents
                    .iter()
                    .map(Self::has_nested_children)
                    .any(|x| x)
        } else {
            true
        }
    }

    pub fn prune(&mut self) {
        self.contents = replace(&mut self.contents, Vec::new())
            .into_iter()
            .filter(Self::has_nested_children)
            .collect();
    }
}

impl fmt::Display for Dir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.format_name())?;

        for (idx, member) in self.contents.iter().enumerate() {
            let mut hanger = PARENT_NTH;
            let mut new_depth = self.nest.clone();

            if idx + 1 == self.contents.len() {
                hanger = PARENT_LAST;
                new_depth.push(false);
            } else {
                new_depth.push(true);
            }

            let adjusted_member = member
                .to_owned()
                .with_nest_level(new_depth)
                .with_format(self.format);

            if self.format.indent {
                let space_before = self
                    .nest
                    .iter()
                    .map(|b| format!("{:4}", if *b { SUPER_DIR } else { ' ' }))
                    .collect::<String>();

                write!(f, "{}{}{}{}", space_before, hanger, INDENT, adjusted_member)?;
            } else {
                write!(f, "{}", adjusted_member)?;
            }
        }

        Ok(())
    }
}

pub enum TreeError {
    IO(io::Error),
    Glob(GlobError),
    Pattern(PatternError),
}

impl From<io::Error> for TreeError {
    fn from(e: io::Error) -> Self {
        TreeError::IO(e)
    }
}

impl From<GlobError> for TreeError {
    fn from(e: GlobError) -> Self {
        TreeError::Glob(e)
    }
}

impl From<PatternError> for TreeError {
    fn from(e: PatternError) -> Self {
        TreeError::Pattern(e)
    }
}

#[derive(Debug, Default)]
struct VcsIgnore {
    black: Vec<Pattern>,
    white: Vec<Pattern>,
}

impl VcsIgnore {
    fn new(file: &PathBuf) -> Result<Self, TreeError> {
        let (mut black, mut white) = (Vec::new(), Vec::new());

        let f = File::open(file)?;
        for line in BufReader::new(f).lines() {
            let line = line?;
            let mut trimmed = line.trim();

            if trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with('/') {
                trimmed = &trimmed[1..];
            }

            if trimmed.starts_with('!') {
                white.push(Self::glob2pat(&trimmed[1..])?);
            } else {
                if trimmed.starts_with("\\#") || trimmed.starts_with("\\!") {
                    trimmed = &trimmed[1..];
                }

                black.push(Self::glob2pat(trimmed)?);
            }
        }

        Ok(Self { white, black })
    }

    fn glob2pat(s: &str) -> Result<Pattern, TreeError> {
        let mut p = PathBuf::new();
        p.push(".");
        p.push(s);
        Ok(Pattern::new(&p.to_string_lossy())?)
    }

    fn find(dir: &PathBuf) -> Option<PathBuf> {
        let mut path_to = dir.to_owned();
        path_to.push(".gitignore");

        if path_to.exists() {
            Some(path_to)
        } else {
            None
        }
    }

    fn in_dir_or_default(dir: &PathBuf) -> Self {
        Self::find(dir).map_or_else(Self::default, |f| {
            Self::new(&f).unwrap_or_else(|_| Self::default())
        })
    }

    fn compose(mut self, other_black: &[Pattern], other_white: &[Pattern]) -> Self {
        self.black.extend_from_slice(other_black);
        self.white.extend_from_slice(other_white);
        self
    }
}
