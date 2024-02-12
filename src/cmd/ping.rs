use crate::cmd::Command;
use crate::frame::Frame;
use crate::{cmd, db, error};
use std::sync::Arc;
use tokio::io::AsyncWrite;

pub struct Ping {
    message: Option<String>,
}

impl Command for Ping {
    async fn apply<T: AsyncWrite + Unpin>(
        &self,
        dest: &mut T,
        _: &Arc<db::State>,
    ) -> std::io::Result<()> {
        let response = if self.message.is_none() {
            Frame::Simple("PONG".into())
        } else {
            Frame::Bulk(self.message.clone().unwrap())
        };
        response.write_to(dest).await
    }

    fn from(frames: Vec<Frame>) -> Result<Self, error::CommandError> {
        let len = frames.len();
        if len > 2 {
            return Err(error::CommandError::Malformed(
                "Ping command requires at most 1 arguments".to_string(),
            ));
        }
        let mut cmd = new();
        if len == 1 {
            return Ok(cmd);
        }
        if let Frame::Bulk(value) = &frames[1] {
            cmd.message = Some(value.into());
        }
        Ok(cmd)
    }
}

/// new creates a new Ping command
pub fn new() -> Ping {
    Ping { message: None }
}
