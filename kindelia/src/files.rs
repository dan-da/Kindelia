use anyhow::Context;
use std::fmt::{Display, Formatter};
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;

/// Represents input from a file or stdin.
#[derive(Debug, Clone)]
pub enum FileInput {
  Stdin,
  Path { path: PathBuf },
}

impl From<PathBuf> for FileInput {
  fn from(path: PathBuf) -> Self {
    FileInput::Path { path }
  }
}

impl FromStr for FileInput {
  type Err = std::convert::Infallible;
  fn from_str(txt: &str) -> Result<Self, Self::Err> {
    let val = if txt == "-" {
      Self::Stdin
    } else {
      let path = txt.into();
      Self::Path { path }
    };
    Ok(val)
  }
}

impl Display for FileInput {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Path { path } => write!(f, "{}", path.display()),
      Self::Stdin => write!(f, "<stdin>"),
    }
  }
}

// TODO: alternative that do not read the whole file immediately
impl FileInput {
  pub fn read_to_string(&self) -> anyhow::Result<String> {
    match self {
      FileInput::Path { path } => {
        // read from file
        std::fs::read_to_string(path)
          .context(format!("Cannot read from '{:?}'", path))
      }
      FileInput::Stdin => {
        // read from stdin
        let mut buff = String::new();
        std::io::stdin()
          .read_to_string(&mut buff)
          .context("Could not read from stdin")?;
        Ok(buff)
      }
    }
  }
}
