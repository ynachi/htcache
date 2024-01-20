use crate::cmd::{get_name, ping, Command};
use crate::frame::Frame;
use crate::{db, error};
use std::io::Write;
use std::sync::Arc;

pub struct Ping {
    message: Option<String>,
}

impl Command for Ping {
    fn apply<T: Write>(&self, dest: &mut T, _: &Arc<db::HTCache>) -> std::io::Result<()> {
        let response = if self.message.is_none() {
            Frame::Simple("PONG".into())
        } else {
            Frame::Bulk(self.message.clone().unwrap())
        };
        response.write_to(dest)
    }

    fn from(frame: &Frame) -> Result<Self, error::CommandError> {
        let cmd_name = get_name(frame)?;
        match frame {
            Frame::Array(content) => {
                if cmd_name.to_ascii_uppercase() != "PING" || content.len() > 2 {
                    return Err(error::CommandError::MalformedPing);
                }

                let mut cmd = new();
                if content.len() == 1 {
                    return Ok(cmd);
                }

                if let Frame::Bulk(value) = &content[1] {
                    cmd.message = Some(value.into());
                }
                Ok(cmd)
            }
            _ => Err(error::CommandError::NotCmdFrame),
        }
    }
}

/// new creates a new Ping command
pub fn new() -> Ping {
    Ping { message: None }
}
