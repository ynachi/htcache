use crate::cmd;
use crate::cmd::Command;
use crate::frame::Frame;
use crate::{db, error};
use std::sync::Arc;
use tokio::io::AsyncWrite;

pub struct Set {
    key: String,
    value: String,
    // ttl: Option<Duration>,
}

impl Command for Set {
    async fn apply<T: AsyncWrite + Unpin>(
        &self,
        dest: &mut T,
        cache: &Arc<db::State>,
    ) -> std::io::Result<()> {
        cache.set_kv(&self.key, &self.value, None);
        let response = Frame::Simple("OK".into());
        response.write_to(dest).await
    }

    fn from(frames: Vec<Frame>) -> Result<Self, error::CommandError> {
        if frames.len() != 3 {
            return Err(error::CommandError::Malformed(
                "SET command requires 2 arguments".to_string(),
            ));
        }
        let mut cmd = new();
        if let Frame::Bulk(value) = &frames[1] {
            cmd.key = value.to_string();
        }
        if let Frame::Bulk(value) = &frames[2] {
            cmd.value = value.to_string();
        }
        Ok(cmd)
    }
}

pub fn new() -> Set {
    Set {
        key: "".to_string(),
        value: "".to_string(),
    }
}
