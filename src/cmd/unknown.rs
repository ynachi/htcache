use crate::cmd::Command;
use crate::error;
use crate::frame::Frame;
use std::fmt::{Display, Formatter};
use std::io::Write;

pub struct Unknown {}

impl Display for Unknown {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Command for Unknown {
    fn apply<T: Write>(&self, dest: &mut T) -> std::io::Result<()> {
        todo!()
    }

    fn from(&mut self, frame: &Frame) -> Result<(), error::CommandError> {
        todo!()
    }
}
