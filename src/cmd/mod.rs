mod get;
pub use get::Get;
mod ping;
pub use ping::Ping;
mod del;
pub use del::Del;
mod set;

pub use set::Set;

use crate::frame::Frame;
use crate::{db, error};
use std::io;
use std::sync::Arc;
use Frame::Bulk;

/// Command represents a htcache command
pub trait Command {
    // apply applies the command
    // @TODO: This method should take DB and Writer as args.
    // Will do after I define them.
    fn apply<T: io::Write>(&self, dest: &mut T, htcache: &Arc<db::HTCache>) -> io::Result<()>;

    /// from read forms the command from a frame
    fn from(frame: &Frame) -> Result<Self, error::CommandError>
    where
        Self: Sized;
}

/// get_name gets the name of the command from the frame.
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

/// check_cmd_frame checks if a cmd frame matches expected command name and arg list
/// (include the command itself).
/// It also returns the content of the frame.
pub fn check_cmd_frame(
    frame: &Frame,
    min_args_len: usize,
    max_args_len: Option<usize>,
    exact_cmd_name: &str,
) -> Result<Vec<Frame>, error::CommandError> {
    let cmd_name = get_name(frame)?;
    match frame {
        Frame::Array(content) => {
            if cmd_name.to_ascii_uppercase() != exact_cmd_name
                || content.len() < min_args_len
                || max_args_len.map_or(false, |max| content.len() > max)
            {
                return Err(error::CommandError::Malformed(cmd_name.to_string()));
            }
            Ok(content.clone())
        }
        _ => Err(error::CommandError::NotCmdFrame),
    }
}
