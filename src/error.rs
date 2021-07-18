use json_dotpath::Error as JsonDotPathError;
use serde_json::Error as SerdeJsonError;
use aes_gcm::Error as EncryptionError;
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
    InvalidKeyLength,
    Encryption,
    Decryption,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(ref err) => err.fmt(f),
            Error::NotFound => write!(f, "Store not found"),
            Error::DotPath(ref err) => err.fmt(f),
            Error::Serde(ref err) => err.fmt(f),
            Error::ConfigDir => write!(f, "Config directory not found"),
            Error::Encryption => write!(f, "Encryption error"),
            Error::InvalidKeyLength => write!(f, "Invalid encryption key length"),
            Error::Decryption => write!(f, "Decryption error"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Io(ref err) => Some(err),
            Error::NotFound => None,
            Error::DotPath(ref err) => Some(err),
            Error::Serde(ref err) => Some(err),
            Error::ConfigDir => None,
            Error::Encryption => None,
            Error::InvalidKeyLength => None,
            Error::Decryption => None,
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

/// A function to convert aes_gcm::Error to Error.
impl From<EncryptionError> for Error {
    fn from(_: EncryptionError) -> Error {
        Error::Encryption
    }
}