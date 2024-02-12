use crate::cmd::Command;
use crate::db::State;
use crate::frame::Frame;
use crate::{cmd, error};
use std::sync::Arc;
use tokio::io::AsyncWrite;

pub struct Get {
    key: String,
}

impl Command for Get {
    async fn apply<T: AsyncWrite + Unpin>(
        &self,
        dest: &mut T,
        cache: &Arc<State>,
    ) -> std::io::Result<()> {
        let response_frame = match cache.get_value_by_key(&self.key) {
            Some(value) => Frame::Bulk(value.to_string()),
            None => Frame::Null,
        };
        response_frame.write_to(dest).await
    }

    fn from(frames: Vec<Frame>) -> Result<Self, error::CommandError> {
        // cmd name is included
        if frames.len() != 2 {
            return Err(error::CommandError::Malformed(
                "GET command requires 1 arguments".to_string(),
            ));
        }
        let mut cmd = new();
        if let Frame::Bulk(value) = &frames[1] {
            cmd.key = value.to_string();
        };
        Ok(cmd)
    }
}

pub fn new() -> Get {
    Get { key: String::new() }
}
