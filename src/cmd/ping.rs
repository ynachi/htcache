use crate::cmd::Command;
use crate::frame::Frame;
use crate::{cmd, db, error};
use std::io::Write;
use std::sync::Arc;

pub struct Ping {
    message: Option<String>,
}

impl Command for Ping {
    fn apply<T: Write>(&self, dest: &mut T, _: &Arc<db::State>) -> std::io::Result<()> {
        let response = if self.message.is_none() {
            Frame::Simple("PONG".into())
        } else {
            Frame::Bulk(self.message.clone().unwrap())
        };
        response.write_to(dest)
    }

    fn from(frame: &Frame) -> Result<Self, error::CommandError> {
        let content = cmd::check_cmd_frame(frame, 1, Some(2), "SET")?;
        let mut cmd = new();
        if content.len() == 1 {
            return Ok(cmd);
        }
        if let Frame::Bulk(value) = &content[1] {
            cmd.message = Some(value.into());
        }
        Ok(cmd)
    }
}

/// new creates a new Ping command
pub fn new() -> Ping {
    Ping { message: None }
}
