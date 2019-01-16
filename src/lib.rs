use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::PathBuf;

const SUPER_DIR: char = '│';
const PARENT_NTH: char = '├';
const PARENT_LAST: char = '└';
const INDENT: &str = "── ";

#[derive(Clone)]
pub struct Dir {
    path: PathBuf,
    contents: Vec<Dir>,
    nest: Vec<bool>,
    format: FormatOpts,
}

impl Dir {
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
            self.path.file_name().unwrap_or(OsStr::new(".")).to_str()
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

impl From<&PathBuf> for Dir {
    fn from(obj: &PathBuf) -> Self {
        if obj.is_dir() {
            Self {
                path: obj.to_owned(),
                nest: Vec::new(),
                contents: fs::read_dir(obj)
                    .unwrap()
                    .into_iter()
                    .map(|e| Self::from(&e.unwrap().path()))
                    .collect::<Vec<_>>(),
                format: FormatOpts::default(),
            }
        } else {
            Self {
                path: obj.to_owned(),
                nest: Vec::new(),
                contents: Vec::new(),
                format: FormatOpts::default(),
            }
        }
    }
}

#[derive(Copy, Clone, Default)]
pub struct FormatOpts {
    pub full_paths: bool,
    pub indent: bool,
    pub quote_names: bool,
}
