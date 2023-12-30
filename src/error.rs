use std::fmt::{Debug, Display, Formatter, Result};
use std::io;
use std::num::ParseIntError;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum FrameError {
    EOF,
    Encoding(io::Error),
    InvalidFrame,
    InvalidType,
    StringFromUTF8(FromUtf8Error),
    IntFromUTF8(ParseIntError),
}

impl Display for FrameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            FrameError::Encoding(err) => write!(f, "error encoding RESP frame: {}", err),
            FrameError::InvalidFrame => write!(f, "RESP frame is malformed"),
            FrameError::InvalidType => write!(f, "wrong RESP frame type, needed another type here"),
            FrameError::EOF => write!(f, "file reached EOF"),
            FrameError::StringFromUTF8(err) => write!(f, "cannot convert bytes to string: {}", err),
            FrameError::IntFromUTF8(err) => write!(f, "cannot convert bytes to int: {}", err),
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

impl From<FromUtf8Error> for FrameError {
    fn from(value: FromUtf8Error) -> Self {
        FrameError::StringFromUTF8(value)
    }
}

impl From<ParseIntError> for FrameError {
    fn from(value: ParseIntError) -> Self {
        FrameError::IntFromUTF8(value)
    }
}

#[derive(Debug)]
pub enum CommandError {
    NotCmdFrame,
    Unknown(String),
    MalformedPing,
    InvalidCmdFrame,
    Connection,
    FrameDecode(FrameError), // this variant is a wrapper of FrameError
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            CommandError::NotCmdFrame => {
                write!(f, "commands are only represented by arrays of frames")
            }
            CommandError::Unknown(cmd_name) => {
                write!(f, "unknown command: {}", cmd_name)
            }
            CommandError::MalformedPing => {
                write!(f, "ping command is invalid: malformed")
            }
            CommandError::InvalidCmdFrame => {
                write!(f, "frame is an array but cannot be a valid command")
            }
            CommandError::Connection => {
                write!(f, "network error: error while writing to network")
            }
            CommandError::FrameDecode(e) => {
                write!(f, "{}", e)
            }
        }
    }
}

// Allow the error to be used with ?
impl std::error::Error for CommandError {}

#[derive(Debug)]
pub enum DatabaseError {
    NoAllocation,
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            DatabaseError::NoAllocation => write!(f, "no capacity allocated to the database"),
        }
    }
}
impl std::error::Error for DatabaseError {}
