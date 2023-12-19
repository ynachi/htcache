use crate::cmd::{get_name, Command};
use crate::error;
use crate::frame::Frame;
use std::fmt::{Display, Formatter};
use std::io::Write;

pub struct Ping {
    message: String,
}

impl Display for Ping {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Command for Ping {
    fn apply<T: Write>(&self, dest: &mut T) -> std::io::Result<()> {
        let response = Frame::Simple(self.message.clone());
        response.write_to(dest)
    }

    fn from(&mut self, frame: &Frame) -> Result<(), error::CommandError> {
        let cmd_name = get_name(frame)?;
        match frame {
            Frame::Array(content) => {
                if cmd_name != "PING" || content.len() > 2 {
                    return Err(error::CommandError::MalformedPing);
                }
                if content.len() == 1 {
                    self.message = "PONG".to_string();
                } else if let Frame::Simple(value) = &content[1] {
                    self.message = value.to_string();
                }
                Ok(())
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
