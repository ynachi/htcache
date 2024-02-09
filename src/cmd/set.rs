use crate::cmd;
use crate::cmd::Command;
use crate::frame::Frame;
use crate::{db, error};
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
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

    fn from(frame: &Frame) -> Result<Self, error::CommandError> {
        let content = cmd::check_cmd_frame(frame, 3, Some(3), "SET")?;
        let mut cmd = new();
        if let Frame::Bulk(value) = &content[1] {
            cmd.key = value.to_string();
        }
        if let Frame::Bulk(value) = &content[2] {
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
