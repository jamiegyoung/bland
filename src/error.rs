use json_dotpath::Error as JsonDotPathError;
use serde_json::Error as SerdeJsonError;
use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Write,
    Read,
    NotFound,
    DotPath(JsonDotPathError),
    Serde(SerdeJsonError),
    ConfigDir,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(ref err) => err.fmt(f),
            Error::Write => write!(f, "Store write error"),
            Error::Read => write!(f, "Store read error"),
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
            Error::Write => None,
            Error::Read => None,
            Error::NotFound => None,
            Error::DotPath(ref err) => err.source(),
            Error::Serde(ref err) => err.source(),
            Error::ConfigDir => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<SerdeJsonError> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::Serde(e)
    }
}

impl From<JsonDotPathError> for Error {
    fn from(e: JsonDotPathError) -> Error {
        Error::DotPath(e)
    }
}
