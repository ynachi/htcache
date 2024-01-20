use crate::cmd::{get_name, Command};
use crate::frame::Frame;
use crate::{db, error};
use std::io::Write;
use std::sync::Arc;

pub struct Set {
    key: String,
    value: String,
}

impl Command for Set {
    fn apply<T: Write>(&self, dest: &mut T, htcache: &Arc<db::HTCache>) -> std::io::Result<()> {
        htcache.set_kv(&self.key, &self.value);
        let response = Frame::Simple("OK".into());
        response.write_to(dest)
    }

    fn from(frame: &Frame) -> Result<Self, error::CommandError> {
        let cmd_name = get_name(frame)?;
        match frame {
            Frame::Array(content) => {
                if cmd_name.to_ascii_uppercase() != "SET" {
                    return Err(error::CommandError::MalformedPing);
                }
                let mut cmd = new();
                if let Frame::Bulk(value) = &content[1] {
                    cmd.key = value.to_string();
                }
                if let Frame::Bulk(value) = &content[2] {
                    cmd.value = value.to_string();
                }
                Ok(cmd)
            }
            _ => Err(error::CommandError::NotCmdFrame),
        }
    }
}

pub fn new() -> Set {
    Set {
        key: "".to_string(),
        value: "".to_string(),
    }
}
