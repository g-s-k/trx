use std::fs::File;
use std::io::{stdout, BufWriter, Result as IOResult, Write};
use std::path::PathBuf;

use glob::{Pattern, PatternError};
use structopt::StructOpt;

use trx::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "trx", about = "A tree command that gets it")]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
#[structopt(rename_all = "kebab-case")]
struct Config {
    // search params
    /// Show hidden files (that start with a `.`)
    #[structopt(short)]
    all: bool,
    /// Show files ignored by git
    #[structopt(long)]
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
    /// Don't descend into directories with more than `n` entries
    #[structopt(long = "filelimit")]
    file_limit: Option<usize>,

    // globs
    /// Glob / literal filenames to match (accepts multiple e.g. -P <first> -P <second>)
    #[structopt(short = "P")]
    keep_pattern: Vec<String>,
    /// Glob / literal names to exclude (accepts multiple e.g. -I <first> -I <second>)
    #[structopt(short = "I")]
    ignore_pattern: Vec<String>,
    /// Ignore case in glob matches
    #[structopt(long)]
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
    /// Print trailing slash and other decorations (like `ls -F`)
    #[structopt(short = "F")]
    decorate_names: bool,
    /// Don't show empty directories
    #[structopt(long = "prune")]
    prune_dirs: bool,
    /// Don't colorize output
    #[structopt(short)]
    no_color: bool,
    /// Print file size in bytes
    #[structopt(short)]
    size: bool, // TODO: add sizes
    /// Print human-readable file size
    #[structopt(short)]
    human_size: bool,
    /// Print file size in SI units
    #[structopt(long = "si")]
    si_size: bool,
    /// Print disk usage per directory
    #[structopt(long = "du")]
    du_size: bool,
    /// Don't show the report at the end of the listing
    #[structopt(long = "noreport")]
    no_report: bool, // TODO: consider adding a report for this to silence
    /// Character set to use in output
    #[structopt(long, default_value = "UTF-8")]
    charset: String, // TODO: determine usefulness of switching charsets
    /// Print the last modification time of each file
    #[structopt(short = "D")]
    mod_time: bool,
    /// Date format string
    #[structopt(long = "timefmt")]
    time_format: Option<String>, // TODO: implement dates

    // output
    /// Send output to a file
    #[structopt(short, parse(from_os_str))]
    output: Option<PathBuf>,
    /// Output as HTML
    #[structopt(short = "H")]
    html_out: bool, // TODO: in the original this takes a value
    /// Set HTML title and header text
    #[structopt(short = "T", requires = "html-out")]
    html_title: Option<String>, // TODO: implement this
    /// Don't include links in HTML output
    #[structopt(long = "nolinks", requires = "html-out")]
    no_links: bool,
    /// Output as JSON
    #[structopt(short = "J", conflicts_with = "html-out")]
    json_out: bool,

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
        decorate: cfg.decorate_names,
        full_paths: cfg.full_paths,
        indent: !cfg.no_indent,
        quote_names: cfg.quote_names,
        html_links: !cfg.no_links,
    });

    if cfg.prune_dirs {
        tree.prune();
    }

    tree.sort_children();

    let output: Box<Write> = if let Some(file) = cfg.output {
        Box::new(File::create(file)?)
    } else {
        Box::new(stdout())
    };

    let mut buffered = BufWriter::new(output);

    if cfg.html_out {
        buffered.write_all(tree.to_html().as_bytes())?;
    } else if cfg.json_out {
        serde_json::to_writer(&mut buffered, &tree)?;
    } else {
        buffered.write_all(tree.to_string().as_bytes())?;
    }

    buffered.write(b"\n")?;

    Ok(())
}
