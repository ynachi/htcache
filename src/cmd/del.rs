use crate::cmd;
use crate::cmd::Command;
use crate::db::State;
use crate::error::CommandError;
use crate::frame::Frame;
use std::sync::Arc;
use tokio::io::AsyncWrite;

pub struct Del {
    keys: Vec<String>,
}

impl Command for Del {
    async fn apply<T: AsyncWrite + Unpin>(
        &self,
        dest: &mut T,
        cache: &Arc<State>,
    ) -> std::io::Result<()> {
        let deleted = cache.delete_entries(&self.keys);
        let response_frame = Frame::Integer(deleted as i64);
        response_frame.write_to(dest).await
    }

    fn from(frame: &Frame) -> Result<Self, CommandError> {
        let content = cmd::check_cmd_frame(frame, 2, None, "DEL")?;
        let mut cmd = new();
        for f in content.iter().skip(1) {
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
