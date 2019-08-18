pub mod collada;

use crate::scene::Scene;
use std::convert::From;
use std::error::Error;
use std::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub struct ImportError {
    message: String,
}

impl Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error during Import: {}", &self.message)
    }
}

impl Error for ImportError {}

// impl From<Error> for ImportError {

// }

pub trait Import {
    fn import(&self) -> Result<Scene, ImportError>;
}
