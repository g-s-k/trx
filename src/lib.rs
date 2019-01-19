#![deny(clippy::pedantic)]

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::mem::replace;
use std::path::PathBuf;

use glob::{MatchOptions, Pattern};

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
    pub positive_patterns: &'a [Pattern],
    pub negative_patterns: &'a [Pattern],
    pub case_insensitive_match: bool,
}

#[derive(Clone, Copy, Default)]
pub struct FormatOpts {
    pub full_paths: bool,
    pub indent: bool,
    pub quote_names: bool,
}

#[derive(Clone)]
pub struct Dir {
    path: PathBuf,
    is_dir: bool,
    contents: Vec<Dir>,
    nest: Vec<bool>,
    format: FormatOpts,
}

impl Dir {
    pub fn from(obj: &PathBuf, cfg: SearchOpts) -> Option<Self> {
        let match_opts = MatchOptions {
            case_sensitive: !cfg.case_insensitive_match,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        };

        for pat in cfg.negative_patterns {
            if pat.matches_path_with(obj, &match_opts) {
                return None;
            }
        }

        let link_contents = obj
            .read_link()
            .map(|e| e.canonicalize().unwrap().starts_with(obj));
        let should_follow_link = cfg.follow_symlinks
            && (!cfg.stay_on_fs || (link_contents.is_ok() && *link_contents.as_ref().unwrap()));

        let (should_recur, max_depth) = match cfg.max_depth {
            Some(n) if n == 0 => (false, None),
            Some(n) => (true, Some(n - 1)),
            None => (true, None),
        };

        if obj.is_dir() && should_recur && (should_follow_link || link_contents.is_err()) {
            let contents = fs::read_dir(obj)
                .unwrap()
                .map(Result::unwrap)
                .filter(|e| !cfg.dirs_only || e.metadata().unwrap().is_dir())
                .filter_map(|e| Self::from(&e.path(), SearchOpts { max_depth, ..cfg }))
                .collect::<Vec<_>>();

            Some(Self {
                path: obj.to_owned(),
                is_dir: true,
                nest: Vec::new(),
                contents,
                format: FormatOpts::default(),
            })
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
                    is_dir: false,
                    nest: Vec::new(),
                    contents: Vec::new(),
                    format: FormatOpts::default(),
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

    fn format_name(&self) -> String {
        let stringified = if self.format.full_paths {
            self.path.to_str()
        } else {
            self.path
                .file_name()
                .unwrap_or_else(|| OsStr::new("."))
                .to_str()
        }
        .unwrap();

        if self.format.quote_names {
            format!("\"{}\"", stringified)
        } else {
            stringified.to_string()
        }
    }

    pub fn sort_children(&mut self) {
        self.contents.sort_unstable_by_key(|v| v.path.clone());
        self.contents.iter_mut().for_each(|c| c.sort_children());
    }

    fn has_nested_children(&self) -> bool {
        !self.is_dir
            || !self.contents.is_empty()
                && self
                    .contents
                    .iter()
                    .map(Self::has_nested_children)
                    .any(|x| x)
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
