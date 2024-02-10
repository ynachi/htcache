use crate::db::State;
use crate::error::CommandError;
use crate::frame::Frame;
use std::sync::Arc;
use tokio::io::AsyncWrite;

pub struct Del {
    keys: Vec<String>,
}

impl Del {
    pub(crate) async fn apply<T: AsyncWrite + Unpin>(
        &self,
        dest: &mut T,
        state: &Arc<State>,
    ) -> std::io::Result<()> {
        let deleted = state.delete_entries(&self.keys);
        let response_frame = Frame::Integer(deleted as i64);
        response_frame.write_to(dest).await
    }

    pub(crate) fn from_vec(frames: &[Frame]) -> Result<Self, CommandError> {
        if frames.len() < 1 {
            return Err(CommandError::Malformed(
                "DEL command requires at least one key".to_string(),
            ));
        }
        let mut cmd = new();
        for f in frames.iter() {
            if let Frame::Bulk(value) = f {
                cmd.keys.push(value.clone())
            }
        }
        Ok(cmd)
    }
}

fn new() -> Del {
    Del { keys: Vec::new() }
}
