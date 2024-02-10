use crate::error;
use crate::frame::Frame;
use tokio::io::AsyncWrite;

pub struct Ping {
    message: Option<String>,
}

impl Ping {
    pub(crate) async fn apply<T: AsyncWrite + Unpin>(&self, dest: &mut T) -> std::io::Result<()> {
        let response = if self.message.is_none() {
            Frame::Simple("PONG".into())
        } else {
            Frame::Bulk(self.message.clone().unwrap())
        };
        response.write_to(dest).await
    }

    pub(crate) fn from_vec(frames: &[Frame]) -> Result<Self, error::CommandError> {
        let len = frames.len();
        if len > 1 {
            return Err(error::CommandError::Malformed(
                "PING command takes zero or one argument".into(),
            ));
        }
        let mut cmd = new();
        if frames.len() == 0 {
            return Ok(cmd);
        }
        if let Frame::Bulk(value) = &frames[0] {
            cmd.message = Some(value.into());
        }
        Ok(cmd)
    }
}

/// new creates a new Ping command
pub fn new() -> Ping {
    Ping { message: None }
}
