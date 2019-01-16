use std::ffi::OsStr;
use std::fs::FileType;
use std::path::PathBuf;

use ignore::overrides::OverrideBuilder;
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

fn main() {
    let cfg = Cli::from_args();
    let current_dir = PathBuf::from(".");
    let dir = cfg.dir.unwrap_or_else(|| current_dir.clone());
    let canonical_dir = dir.canonicalize().unwrap();

    // glob patterns
    let mut over = OverrideBuilder::new(&dir);

    if !cfg.keep_pattern.is_empty() {
        over.add("!*").unwrap();
    }

    for glob in cfg.keep_pattern {
        over.add(&glob).unwrap();
    }

    for glob in cfg.ignore_pattern {
        over.add(&format!("!{}", glob)).unwrap();
    }

    // find the files
    for entry in WalkBuilder::new(&dir)
        .max_depth(cfg.max_depth)
        .hidden(!cfg.all)
        .follow_links(cfg.symlinks)
        .overrides(over.build().unwrap())
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

                let mut out = e.into_path();

                let name = if cfg.full_paths && out != dir {
                    let tmp = out
                        .canonicalize()
                        .unwrap()
                        .strip_prefix(&canonical_dir)
                        .unwrap()
                        .to_path_buf();
                    out = current_dir.clone();
                    out.push(tmp);
                    out.to_str().unwrap()
                } else {
                    out.file_name()
                        .unwrap_or_else(|| OsStr::new("."))
                        .to_str()
                        .unwrap()
                };

                if cfg.quote_names {
                    println!("\"{}\"", name);
                } else {
                    println!("{}", name);
                }
            }
            Err(e) => eprintln!("ERROR: {}", e),
        }
    }
}
