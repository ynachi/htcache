use crate::cmd::Command;
use crate::db::State;
use crate::error;
use crate::frame::Frame;
use std::io::{BufWriter, Write};
use std::sync::Arc;

pub struct Set {
    key: String,
    value: String,
    // ttl: Option<Duration>,
}

impl Command for Set {
    fn apply<T: Write>(&self, dest: &mut BufWriter<T>, cache: &Arc<State>) -> std::io::Result<()> {
        cache.set_kv(&self.key, &self.value, None);
        let response = Frame::Simple("OK".into());
        response.write_to(dest)
    }

    fn from(frames: Vec<Frame>) -> Result<Self, error::CommandError> {
        if frames.len() != 3 {
            return Err(error::CommandError::Malformed(
                "SET command requires 2 arguments".to_string(),
            ));
        }
        let mut cmd = new();
        if let Frame::Bulk(value) = &frames[1] {
            cmd.key = value.to_string();
        }
        if let Frame::Bulk(value) = &frames[2] {
            cmd.value = value.to_string();
        }
        Ok(cmd)
    }
}

pub fn new() -> Set {
    Set {
        key: "".to_string(),
        value: "".to_string(),
    }
}
