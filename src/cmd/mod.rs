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
use tokio::io::AsyncWrite;
use Frame::Bulk;

/// Command represents a htcache command
pub(crate) trait Command {
    // apply applies the command
    // @TODO: This method should take DB and Writer as args.
    // Will do after I define them.
    async fn apply<T: AsyncWrite + Unpin>(
        &self,
        dest: &mut T,
        cache: &Arc<db::State>,
    ) -> io::Result<()>;

    /// from read forms the command from a frame
    fn from(frames: Vec<Frame>) -> Result<Self, error::CommandError>
    where
        Self: Sized;
}

/// parse_frame checks a frame and extracts its content, including the command name.
pub fn parse_frame(frame: Frame) -> Result<(String, Vec<Frame>), error::CommandError> {
    // commands are only expressed as Frame arrays of bulks
    match frame {
        Frame::Array(frames) => {
            if frames.is_empty() {
                return Err(error::CommandError::InvalidCmdFrame);
            }
            match &frames[0] {
                Bulk(cmd_name) => Ok((cmd_name.to_uppercase(), frames)),
                _ => Err(error::CommandError::InvalidCmdFrame),
            }
        }
        _ => Err(error::CommandError::NotCmdFrame),
    }
}
