use std::fmt::{Debug, Display, Formatter, Result};
use std::io;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub(crate) enum FrameError {
    EOF,
    Encoding(io::Error),
    InvalidFrame,
    InvalidType,
    InvalidUTF8(FromUtf8Error),
}

impl Display for FrameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            FrameError::Encoding(err) => write!(f, "error encoding RESP frame: {}", err),
            FrameError::InvalidFrame => write!(f, "RESP frame is malformed"),
            FrameError::InvalidType => write!(f, "wrong RESP frame type, needed another type here"),
            FrameError::EOF => write!(f, "file reached EOF"),
            FrameError::InvalidUTF8(err) => write!(f, "cannot convert bytes to string: {}", err),
        }
    }
}

// Allow the error to be used with ?
impl std::error::Error for FrameError {}

// Convert io::Error to FrameError::Encoding
impl From<io::Error> for FrameError {
    fn from(err: io::Error) -> Self {
        FrameError::Encoding(err)
    }
}
