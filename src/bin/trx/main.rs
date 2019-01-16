use std::io::Result;
use std::path::PathBuf;

use structopt::StructOpt;

use trx::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "trx", about = "A tree command that gets it")]
struct Config {
    #[structopt(short)]
    all: bool,
    #[structopt(short)]
    directories: bool,
    #[structopt(short = "l")]
    symlinks: bool,
    #[structopt(short = "x")]
    stay_on_fs: bool,
    #[structopt(short = "L")]
    max_depth: Option<usize>,
    #[structopt(short)]
    full_paths: bool,
    #[structopt(short = "Q")]
    quote_names: bool,
    #[structopt(short = "i")]
    no_indent: bool,
    #[structopt(short = "P")]
    keep_pattern: Vec<String>,
    #[structopt(short = "I")]
    ignore_pattern: Vec<String>,
    #[structopt(parse(from_os_str))]
    dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cfg = Config::from_args();
    let current_dir = PathBuf::from(".");
    let dir = cfg.dir.as_ref().unwrap_or(&current_dir);
    let mut tree = Dir::from(dir).with_format(FormatOpts {
        full_paths: cfg.full_paths,
        indent: !cfg.no_indent,
        quote_names: cfg.quote_names,
    });

    tree.sort_children();

    println!("{}", tree);

    Ok(())
}
