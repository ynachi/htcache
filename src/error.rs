use std::fmt::{Debug, Display, Formatter, Result};
use std::io;

#[derive(Debug)]
pub(crate) enum FrameError {
    Encoding(io::Error),
    Decoding(io::Error),
    InvalidFrame(String),
    InvalidType(String),
}

impl Display for FrameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            FrameError::Encoding(err) => write!(f, "error encoding RESP frame: {}", err),
            FrameError::Decoding(err) => write!(f, "error decoding RESP frame: {}", err),
            FrameError::InvalidFrame(msg) => write!(f, "invalid RESP frame: {}", msg),
            FrameError::InvalidType(msg) => write!(f, "invalid RESP type: {}", msg),
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
