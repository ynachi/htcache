mod get;
pub use get::Get;
mod ping;
pub use ping::Ping;
mod del;
pub use del::Del;
mod set;

pub use set::Set;

use crate::db;
use crate::error::CommandError;
use crate::frame::Frame;
use std::io;
use std::sync::Arc;
use tokio::io::AsyncWrite;
use Frame::Bulk;

// We do not use trait for command because one of the functions requires to be async.
// Trait is not recommended for now when asynch is involved.
pub enum Command {
    Get(Get),
    Set(Set),
    Del(Del),
    Ping(Ping),
}

impl Command {
    pub fn from_frame(frame: Frame) -> Result<Command, CommandError> {
        // Get frame name and content in one shot.
        // This is to avoid unwrapping the frame many times.
        // We need to know the frame name to call the appropriate from_array()
        // method.
        let (cmd_name, frames) = match frame {
            Frame::Array(frames) => {
                if frames.is_empty() {
                    return Err(CommandError::InvalidCmdFrame);
                }
                match &frames[0] {
                    Bulk(cmd_name) => (cmd_name.to_uppercase(), frames),
                    _ => return Err(CommandError::InvalidCmdFrame),
                }
            }
            _ => return Err(CommandError::NotCmdFrame),
        };

        match &cmd_name[..] {
            "GET" => Ok(Command::Get(Get::from_vec(&frames[1..])?)),
            "SET" => Ok(Command::Set(Set::from_vec(&frames[1..])?)),
            "DEL" => Ok(Command::Del(Del::from_vec(&frames[1..])?)),
            "PING" => Ok(Command::Ping(Ping::from_vec(&frames[1..])?)),
            _ => return Err(CommandError::Unknown(cmd_name)),
        }
    }

    pub async fn apply<T: AsyncWrite + Unpin>(
        &self,
        dest: &mut T,
        state: &Arc<db::State>,
    ) -> io::Result<()> {
        match self {
            Command::Get(cmd) => cmd.apply(dest, state).await,
            Command::Set(cmd) => cmd.apply(dest, state).await,
            Command::Del(cmd) => cmd.apply(dest, state).await,
            Command::Ping(cmd) => cmd.apply(dest).await,
        }
    }
}

/// get_name gets the name of the command from the frame.
pub fn get_name(frame: &Frame) -> Result<String, CommandError> {
    // commands are only expressed as Frame arrays of bulks
    match frame {
        Frame::Array(frames) => {
            if frames.is_empty() {
                return Err(CommandError::InvalidCmdFrame);
            }
            match &frames[0] {
                Bulk(cmd_name) => Ok(cmd_name.to_uppercase()),
                _ => Err(CommandError::InvalidCmdFrame),
            }
        }
        _ => Err(CommandError::NotCmdFrame),
    }
}
