use json_dotpath::Error as JsonDotPathError;
use serde_json::Error as SerdeJsonError;
use std::{error, fmt, io};

/// The `Error` type is an enum for all errors that can be thrown by this library.
#[derive(Debug)]
pub enum Error {
    /// `Io` errors are errors that occur when reading from or writing to a file.
    Io(io::Error),
    /// `NotFound` errors are errors that occur when a requested path is not found.
    NotFound,
    /// `DotPath` errors are errors that occur when using the `JsonDotPath` library.
    DotPath(JsonDotPathError),
    /// `SerdeJson` errors are errors that occur when using the `SerdeJson` library.
    Serde(SerdeJsonError),
    /// `ConfigDir` errors are errors that occur when locating the config directory. 
    ConfigDir,
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(ref err) => err.fmt(f),
            Error::NotFound => write!(f, "Store not found"),
            Error::DotPath(err) => err.fmt(f),
            Error::Serde(err) => err.fmt(f),
            Error::ConfigDir => write!(f, "Config directory not found"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Io(ref err) => err.source(),
            Error::NotFound => None,
            Error::DotPath(ref err) => err.source(),
            Error::Serde(ref err) => err.source(),
            Error::ConfigDir => None,
        }
    }
}

/// A function to convert io::Error to Error.
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

/// A function to convert serde_json::Error to Error.
impl From<SerdeJsonError> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::Serde(e)
    }
}

/// A function to convert json_dotpath::Error to Error.
impl From<JsonDotPathError> for Error {
    fn from(e: JsonDotPathError) -> Error {
        Error::DotPath(e)
    }
}
