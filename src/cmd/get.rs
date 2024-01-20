use crate::cmd::Command;
use crate::db::HTCache;
use crate::frame::Frame;
use crate::{cmd, error};
use std::io::Write;
use std::sync::Arc;

pub struct Get {
    key: String,
}

impl Command for Get {
    fn apply<T: Write>(&self, dest: &mut T, htcache: &Arc<HTCache>) -> std::io::Result<()> {
        let response_frame = match htcache.get_value_for_key(&self.key) {
            Some(value) => Frame::Bulk(value.to_string()),
            None => Frame::Null,
        };
        response_frame.write_to(dest)
    }

    fn from(frame: &Frame) -> Result<Self, error::CommandError> {
        let content = cmd::check_cmd_frame(frame, 2, Some(2), "GET")?;
        let mut cmd = new();
        if let Frame::Bulk(value) = &content[1] {
            cmd.key = value.to_string();
        };
        Ok(cmd)
    }
}

pub fn new() -> Get {
    Get { key: String::new() }
}
