#[cfg(feature = "crypto")]
use aes_gcm::Error as EncryptionError;
use json_dotpath::Error as JsonDotPathError;
use serde_json::Error as SerdeJsonError;
use std::string::FromUtf8Error;
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
    #[cfg(feature = "crypto")]
    InvalidKeyLength,
    #[cfg(feature = "crypto")]
    Encryption,
    #[cfg(feature = "crypto")]
    Decryption,
    FromUTF8Error(FromUtf8Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // TODO: check this
            Error::Io(ref err) => err.fmt(f),
            Error::NotFound => write!(f, "Store not found"),
            Error::DotPath(ref err) => err.fmt(f),
            Error::Serde(ref err) => err.fmt(f),
            Error::ConfigDir => write!(f, "Config directory not found"),
            #[cfg(feature = "crypto")]
            Error::Encryption => write!(f, "Encryption error"),
            #[cfg(feature = "crypto")]
            Error::InvalidKeyLength => write!(f, "Invalid encryption key length"),
            #[cfg(feature = "crypto")]
            Error::Decryption => write!(f, "Decryption error"),
            Error::FromUTF8Error(ref err) => err.fmt(f),
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
            #[cfg(feature = "crypto")]
            Error::Encryption => None,
            #[cfg(feature = "crypto")]
            Error::InvalidKeyLength => None,
            #[cfg(feature = "crypto")]
            Error::Decryption => None,
            Error::FromUTF8Error(ref err) => Some(err),
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
#[cfg(feature = "crypto")]
impl From<EncryptionError> for Error {
    fn from(_: EncryptionError) -> Error {
        Error::Encryption
    }
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Error {
        Error::FromUTF8Error(e)
    }
}
