use crate::frame::Frame;
use crate::{db, error};
use std::sync::Arc;
use tokio::io::AsyncWrite;

pub struct Set {
    key: String,
    value: String,
    // ttl: Option<Duration>,
}

impl Set {
    pub(crate) async fn apply<T: AsyncWrite + Unpin>(
        &self,
        dest: &mut T,
        state: &Arc<db::State>,
    ) -> std::io::Result<()> {
        state.set_kv(&self.key, &self.value, None);
        let response = Frame::Simple("OK".into());
        response.write_to(dest).await
    }

    pub(crate) fn from_vec(frames: &[Frame]) -> Result<Self, error::CommandError> {
        // the command name is par of the frame
        if frames.len() != 2 {
            return Err(error::CommandError::Malformed(
                "SET command requires 2 arguments".to_string(),
            ));
        }
        let mut cmd = new();
        if let Frame::Bulk(value) = &frames[0] {
            cmd.key = value.to_string();
        }
        if let Frame::Bulk(value) = &frames[1] {
            cmd.value = value.to_string();
        }
        Ok(cmd)
    }
}

pub fn new() -> Set {
    Set {
        key: String::new(),
        value: String::new(),
    }
}
