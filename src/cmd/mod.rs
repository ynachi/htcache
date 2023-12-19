pub mod ping;
pub mod set;
mod unknown;

use crate::cmd::ping::Ping;
use crate::cmd::unknown::Unknown;
use crate::frame::Frame;
use crate::{connection, error};
use std::fmt::Display;
use std::io;

/// Command represents a redisy command
pub trait Command: Display {
    // apply applies the command
    // @TODO: This method should take DB and Writer as args.
    // Will do after I define them.
    fn apply<T: io::Write>(&self, dest: &mut T) -> io::Result<()>;

    /// from read forms the command from a frame
    fn from(&mut self, frame: &Frame) -> Result<(), error::CommandError>;
}

/// CommandType is an enum which defines command types.
pub enum CommandType {
    Ping(Ping),
    Unknown(Unknown),
}

/// create_command creates a command based on its name
pub fn create_command(name: &str) -> Option<CommandType> {
    match name {
        "PING" => Some(CommandType::Ping(ping::new())),
        _ => None,
    }
}

/// get_name gets the name of the command from the frame
pub fn get_name(frame: &Frame) -> Result<String, error::CommandError> {
    // commands are only expressed as Frame arrays of bulks
    match frame {
        Frame::Array(frames) => {
            if frames.len() == 0 {
                return Err(error::CommandError::InvalidCmdFrame);
            }
            return match &frames[0] {
                Frame::Bulk(cmd_name) => Ok(cmd_name.clone()),
                _ => Err(error::CommandError::InvalidCmdFrame),
            };
        }
        _ => Err(error::CommandError::NotCmdFrame),
    }
}

/// run runs a command given by its type and frame. We need to issue the frame to
/// construct the full command from it.
pub fn run(command_type: CommandType, frame: Frame, conn: &mut connection::Connection) {
    match command_type {
        CommandType::Ping(mut cmd) => {
            if let Err(e) = cmd.from(&frame) {
                conn.send_error(&e);
            }
            cmd.apply();
        }
        CommandType::Unknown(mut cmd) => {
            if let Err(e) = cmd.from(&frame) {
                conn.send_error(&e);
            }
        }
    }
}
