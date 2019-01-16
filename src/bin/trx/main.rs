use std::ffi::OsStr;
use std::fs::FileType;
use std::path::{Path, PathBuf};

use ignore::overrides::{Override, OverrideBuilder};
use ignore::WalkBuilder;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "trx", about = "A tree command that gets it")]
struct Cli {
    #[structopt(short)]
    all: bool,
    #[structopt(short)]
    directories: bool,
    #[structopt(short = "l")]
    symlinks: bool,
    #[structopt(short)]
    full_paths: bool,
    #[structopt(short = "x")]
    stay_on_fs: bool,
    #[structopt(short = "L")]
    max_depth: Option<usize>,
    #[structopt(short = "Q")]
    quote_names: bool,
    #[structopt(short = "P")]
    keep_pattern: Vec<String>,
    #[structopt(short = "I")]
    ignore_pattern: Vec<String>,
    #[structopt(parse(from_os_str))]
    dir: Option<PathBuf>,
}

fn build_override(config: &Cli, root: &Path) -> Override {
    let mut over = OverrideBuilder::new(root);

    if !config.keep_pattern.is_empty() {
        over.add("!*").unwrap();
    }

    for glob in config.keep_pattern.iter() {
        over.add(&glob).unwrap();
    }

    for glob in config.ignore_pattern.iter() {
        over.add(&format!("!{}", glob)).unwrap();
    }

    over.build().unwrap()
}

fn format_entry(entry: &Path, root: &Path, config: &Cli) -> String {
    let mut new = PathBuf::from(".");

    let name = if config.full_paths && entry != root {
        let tmp = entry
            .canonicalize()
            .unwrap()
            .strip_prefix(&root.canonicalize().unwrap())
            .unwrap()
            .to_path_buf();
        new.push(tmp);
        new.to_str().unwrap()
    } else {
        entry
            .file_name()
            .unwrap_or_else(|| OsStr::new("."))
            .to_str()
            .unwrap()
    };

    if config.quote_names {
        format!("\"{}\"", name)
    } else {
        name.to_string()
    }
}

fn main() {
    let cfg = Cli::from_args();
    let current_dir = PathBuf::from(".");
    let dir = cfg.dir.as_ref().unwrap_or(&current_dir);

    // find the files
    for entry in WalkBuilder::new(&dir)
        .max_depth(cfg.max_depth)
        .hidden(!cfg.all)
        .follow_links(cfg.symlinks)
        .overrides(build_override(&cfg, &dir))
        .build()
    {
        match entry {
            Ok(e) => {
                if cfg.directories
                    && e.file_type()
                        .as_ref()
                        .map(FileType::is_file)
                        .unwrap_or(false)
                {
                    continue;
                }

                // format it
                let name = format_entry(e.path(), &dir, &cfg);

                // print it
                println!("{}", name);
            }
            Err(e) => eprintln!("ERROR: {}", e),
        }
    }
}
