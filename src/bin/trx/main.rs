use std::fs::FileType;
use std::path::PathBuf;

use ignore::WalkBuilder;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "trx", about = "A tree command that gets it")]
struct Cli {
    #[structopt(short = "a")]
    all: bool,
    #[structopt(short = "d")]
    directories: bool,
    #[structopt(short = "l")]
    symlinks: bool,
    #[structopt(short = "f")]
    full_paths: bool,
    #[structopt(short = "x")]
    stay_on_fs: bool,
    #[structopt(short = "L")]
    max_depth: Option<usize>,
    #[structopt(parse(from_os_str))]
    dir: Option<PathBuf>,
}

fn main() {
    let cfg = Cli::from_args();
    let dir = cfg.dir.unwrap_or(PathBuf::from("."));

    for entry in WalkBuilder::new(dir)
        .max_depth(cfg.max_depth)
        .hidden(!cfg.all)
        .follow_links(cfg.symlinks)
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
                if cfg.full_paths {
                    println!("{}", e.path().canonicalize().unwrap().display());
                } else {
                    println!("{}", e.path().display());
                }
            }
            Err(e) => eprintln!("ERROR: {}", e),
        }
    }
}
