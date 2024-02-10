use crate::db::State;
use crate::error;
use crate::frame::Frame;
use std::sync::Arc;
use tokio::io::AsyncWrite;

pub struct Get {
    key: String,
}

impl Get {
    pub(crate) async fn apply<T: AsyncWrite + Unpin>(
        &self,
        dest: &mut T,
        state: &Arc<State>,
    ) -> std::io::Result<()> {
        let response_frame = match state.get_value_by_key(&self.key) {
            Some(value) => Frame::Bulk(value.to_string()),
            None => Frame::Null,
        };
        response_frame.write_to(dest).await
    }

    pub(crate) fn from_vec(frames: &[Frame]) -> Result<Self, error::CommandError> {
        // we implement a basic GET command for now.
        if frames.len() != 1 {
            return Err(error::CommandError::Malformed(
                "GET command requires 1 arguments".to_string(),
            ));
        }
        let mut cmd = new();
        if let Frame::Bulk(value) = &frames[0] {
            cmd.key = value.to_string();
        };
        Ok(cmd)
    }
}

pub fn new() -> Get {
    Get { key: String::new() }
}
