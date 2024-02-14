use crate::cmd::Command;
use crate::db::State;
use crate::error::CommandError;
use crate::frame::Frame;
use std::io::{BufWriter, Write};
use std::sync::Arc;

pub struct Del {
    keys: Vec<String>,
}

impl Command for Del {
    fn apply<T: Write>(&self, dest: &mut BufWriter<T>, cache: &Arc<State>) -> std::io::Result<()> {
        let deleted = cache.delete_entries(&self.keys);
        let response_frame = Frame::Integer(deleted as i64);
        response_frame.write_to(dest)
    }

    fn from(frames: Vec<Frame>) -> Result<Self, CommandError> {
        if frames.len() < 2 {
            return Err(CommandError::Malformed(
                "DEL command requires at least one key".to_string(),
            ));
        }
        let mut cmd = new();
        // skip the command name
        for f in frames.iter().skip(1) {
            if let Frame::Bulk(value) = f {
                cmd.keys.push(value.clone())
            }
        }
        Ok(cmd)
    }
}

fn new() -> Del {
    Del { keys: Vec::new() }
}
