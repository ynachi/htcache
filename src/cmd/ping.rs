use crate::cmd::Command;
use crate::db::State;
use crate::error;
use crate::frame::Frame;
use std::io::{BufWriter, Write};
use std::sync::Arc;

pub struct Ping {
    message: Option<String>,
}

impl Command for Ping {
    fn apply<T: Write>(&self, dest: &mut BufWriter<T>, _: &Arc<State>) -> std::io::Result<()> {
        let response = if self.message.is_none() {
            Frame::Simple("PONG".into())
        } else {
            Frame::Bulk(self.message.clone().unwrap())
        };
        response.write_to(dest)
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
