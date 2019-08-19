mod blender;
mod collada;

pub use blender::Blender;
pub use collada::Collada;

use crate::scene::Scene;
use std::error::Error;
use std::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub struct ImportError {
    message: String,
}

impl Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error during import: {}", &self.message)
    }
}

impl From<String> for ImportError {
    fn from(message: String) -> ImportError {
        ImportError { message }
    }
}

impl From<&str> for ImportError {
    fn from(message: &str) -> ImportError {
        ImportError { message: message.to_owned() }
    }
}

impl Error for ImportError {}

pub trait Import {
    fn import(&self) -> Result<Scene, ImportError>;
}
