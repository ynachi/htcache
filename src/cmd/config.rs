use crate::cmd::{get_name, Command};
use crate::error;
use crate::frame::Frame;
use std::io::Write;

// @TODO dummy implementation of config get to use redis benchmark tool
pub struct Config {
    options: String,
}

impl Command for Config {
    fn apply<T: Write>(&self, dest: &mut T) -> std::io::Result<()> {
        let mut response = Frame::map();
        if self.options == "save" {
            response
                .add_map_frame(
                    Frame::Bulk("save".into()),
                    Frame::Bulk("3600 1 300 100 60 10000".into()),
                )
                .unwrap();
        } else {
            response
                .add_map_frame(Frame::Bulk("appendonly".into()), Frame::Bulk("no".into()))
                .unwrap();
        }
        response.write_to(dest)
    }

    fn from(&mut self, frame: &Frame) -> Result<(), error::CommandError> {
        let cmd_name = get_name(frame)?;
        match frame {
            Frame::Array(content) => {
                if cmd_name != "CONFIG" {
                    return Err(error::CommandError::MalformedPing);
                }
                if let Frame::Bulk(value) = &content[2] {
                    self.options = value.to_string();
                }
                Ok(())
            }
            _ => Err(error::CommandError::NotCmdFrame),
        }
    }
}

pub fn new() -> Config {
    Config {
        options: "".to_string(),
    }
}
