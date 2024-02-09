use std::fmt::{Debug, Display, Formatter, Result};
use std::io::ErrorKind;
use std::net::Shutdown::Write;
use std::num::ParseIntError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::{fmt, io};

#[derive(Debug)]
pub enum FrameError {
    EOF,
    Encoding(io::Error),
    InvalidFrame,
    InvalidType,
    Incomplete,
    StringFromUTF8(FromUtf8Error),
    StrFromUTF8(std::str::Utf8Error),
    IntFromUTF8(ParseIntError),
    UnexpectedEOF,
    ConnectionReset,
    ConnectionRead(io::Error),
}

impl Display for FrameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            FrameError::Encoding(err) => write!(f, "error encoding RESP frame: {}", err),
            FrameError::InvalidFrame => write!(f, "RESP frame is malformed"),
            FrameError::InvalidType => write!(f, "wrong RESP frame type, needed another type here"),
            FrameError::EOF => write!(f, "file reached EOF"),
            FrameError::UnexpectedEOF => write!(f, "connection abruptly closed"),
            FrameError::StringFromUTF8(err) => write!(f, "cannot convert bytes to string: {}", err),
            FrameError::IntFromUTF8(err) => write!(f, "cannot convert bytes to int: {}", err),
            FrameError::ConnectionReset => write!(f, "connection reset by peer"),
            FrameError::ConnectionRead(err) => {
                write!(f, "generic error while reading on connection: {}", err)
            }
            FrameError::Incomplete => write!(f, "frame is incomplete"),
            FrameError::StrFromUTF8(err) => write!(f, "cannot convert bytes to &str: {}", err),
        }
    }
}

// Allow the error to be used with ?
impl std::error::Error for FrameError {}

// Convert io::Error to FrameError::Encoding
impl From<io::Error> for FrameError {
    fn from(err: io::Error) -> Self {
        if err.kind() == ErrorKind::UnexpectedEof {
            return FrameError::EOF;
        }
        FrameError::Encoding(err)
    }
}

impl From<FromUtf8Error> for FrameError {
    fn from(value: FromUtf8Error) -> Self {
        FrameError::StringFromUTF8(value)
    }
}

impl From<Utf8Error> for FrameError {
    fn from(value: Utf8Error) -> Self {
        FrameError::StrFromUTF8(value)
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
    Malformed(String), // string is command name
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
            CommandError::Malformed(name) => {
                write!(f, "'{}' command is invalid: malformed", name)
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

// Combine command and Frame errors.
// This is required because while processing commands, both error types could occur.
pub enum HandleCommandError {
    Frame(FrameError),
    Command(CommandError),
}

impl From<FrameError> for HandleCommandError {
    fn from(error: FrameError) -> Self {
        HandleCommandError::Frame(error)
    }
}

impl From<CommandError> for HandleCommandError {
    fn from(error: CommandError) -> Self {
        HandleCommandError::Command(error)
    }
}

impl Display for HandleCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleCommandError::Frame(err) => write!(f, "{}", err),
            HandleCommandError::Command(err) => write!(f, "{}", err),
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
