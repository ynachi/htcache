pub mod config;
pub mod ping;
pub mod set;

use crate::error;
use crate::frame::Frame;
use std::io;
use Frame::Bulk;

/// Command represents a htcache command
pub trait Command {
    // apply applies the command
    // @TODO: This method should take DB and Writer as args.
    // Will do after I define them.
    fn apply<T: io::Write>(&self, dest: &mut T) -> io::Result<()>;

    /// from read forms the command from a frame
    fn from(frame: &Frame) -> Result<Self, error::CommandError>
    where
        Self: Sized;
}

/// get_name gets the name of the command from the frame
pub fn get_name(frame: &Frame) -> Result<String, error::CommandError> {
    // commands are only expressed as Frame arrays of bulks
    match frame {
        Frame::Array(frames) => {
            if frames.is_empty() {
                return Err(error::CommandError::InvalidCmdFrame);
            }
            match &frames[0] {
                Bulk(cmd_name) => Ok(cmd_name.to_uppercase()),
                _ => Err(error::CommandError::InvalidCmdFrame),
            }
        }
        _ => Err(error::CommandError::NotCmdFrame),
    }
}
