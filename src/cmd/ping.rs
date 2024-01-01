use crate::cmd::{get_name, Command};
use crate::error;
use crate::frame::Frame;
use std::io::Write;

pub struct Ping {
    message: String,
}

impl Command for Ping {
    fn apply<T: Write>(&self, dest: &mut T) -> std::io::Result<()> {
        let response = if self.message == "PONG" {
            Frame::Simple(self.message.clone())
        } else {
            Frame::Bulk(self.message.clone())
        };
        response.write_to(dest)
    }

    fn from(frame: &Frame) -> Result<Self, error::CommandError> {
        let cmd_name = get_name(frame)?;
        let mut message = String::new();
        match frame {
            Frame::Array(content) => {
                if cmd_name.to_ascii_uppercase() != "PING" || content.len() > 2 {
                    return Err(error::CommandError::MalformedPing);
                }
                if content.len() == 1 {
                    message = "PONG".into();
                } else if let Frame::Bulk(value) = &content[1] {
                    message = value.into();
                }
                Ok(Ping { message })
            }
            _ => Err(error::CommandError::NotCmdFrame),
        }
    }
}

impl Ping {
    /// set_message sets the ping message
    pub fn set_message(&mut self, msg: String) {
        self.message = msg
    }
}

/// new creates a new Ping command
pub fn new() -> Ping {
    Ping {
        message: "".to_string(),
    }
}
