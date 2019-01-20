use std::io::Result as IOResult;
use std::path::PathBuf;

use glob::{Pattern, PatternError};
use structopt::StructOpt;

use trx::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "trx", about = "A tree command that gets it")]
struct Config {
    // search params
    /// Show hidden files (that start with a `.`)
    #[structopt(short)]
    all: bool,
    /// Show files ignored by git
    #[structopt(long = "no-ignore-vcs")]
    no_ignore_vcs: bool,
    /// Only show directories
    #[structopt(short)]
    directories: bool,
    /// Follow symlinks
    #[structopt(short = "l")]
    symlinks: bool,
    /// Don't follow symlinks off of this filesystem
    #[structopt(short = "x")]
    stay_on_fs: bool,
    /// Maximum depth to recur to (infinite if unspecified)
    #[structopt(short = "L")]
    max_depth: Option<usize>,

    // globs
    /// Glob / literal filenames to match (accepts multiple e.g. -P <first> -P <second>)
    #[structopt(short = "P")]
    keep_pattern: Vec<String>,
    /// Glob / literal names to exclude (accepts multiple e.g. -I <first> -I <second>)
    #[structopt(short = "I")]
    ignore_pattern: Vec<String>,
    /// Ignore case in glob matches
    #[structopt(long = "case-insensitive")]
    case_insensitive: bool,

    // formatting options
    /// Print full paths
    #[structopt(short)]
    full_paths: bool,
    /// Print each path in double-quotes
    #[structopt(short = "Q")]
    quote_names: bool,
    /// Don't indent output (like `find`)
    #[structopt(short = "i")]
    no_indent: bool,
    /// Don't show empty directories
    #[structopt(long = "prune")]
    prune_dirs: bool,
    /// Don't colorize output
    #[structopt(short)]
    no_color: bool,
    /// Print file size
    #[structopt(short)]
    size: bool,
    /// Print human-readable file size
    #[structopt(short)]
    human_size: bool,

    /// The directory to start in
    #[structopt(parse(from_os_str))]
    dir: Option<PathBuf>,
}

fn pattern_ify(v: Vec<String>) -> Result<Vec<Pattern>, PatternError> {
    let mut out = Vec::new();

    for s in v.iter() {
        out.push(Pattern::new(&format!("**/{}", s))?);
    }

    Ok(out)
}

fn main() -> IOResult<()> {
    let cfg = Config::from_args();
    let current_dir = PathBuf::from(".");
    let dir = cfg.dir.as_ref().unwrap_or(&current_dir);

    let positive = match pattern_ify(cfg.keep_pattern) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(1);
        }
    };
    let negative = match pattern_ify(cfg.ignore_pattern) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(1);
        }
    };

    let search_opts = SearchOpts {
        show_hidden: cfg.all,
        dirs_only: cfg.directories,
        follow_symlinks: cfg.symlinks,
        max_depth: cfg.max_depth,
        stay_on_fs: cfg.stay_on_fs,
        use_gitignores: !cfg.no_ignore_vcs,
        positive_patterns: &positive,
        negative_patterns: &negative,
        case_insensitive_match: cfg.case_insensitive,
        ..Default::default()
    };

    let result = if let Some(t) = Dir::from(dir, search_opts) {
        t
    } else {
        eprintln!("No files matched the given parameters.");
        std::process::exit(0);
    };

    let mut tree = result.with_format(FormatOpts {
        colorize: !cfg.no_color,
        full_paths: cfg.full_paths,
        indent: !cfg.no_indent,
        quote_names: cfg.quote_names,
    });

    if cfg.prune_dirs {
        tree.prune();
    }

    tree.sort_children();

    println!("{}", tree);

    Ok(())
}
