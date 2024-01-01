// use crate::cmd::{get_name, Command};
// use crate::error;
// use crate::error::CommandError;
// use crate::frame::Frame;
// use std::io::Write;
//
// pub struct Set {
//     key: String,
//     value: String,
// }
//
// impl Command for Set {
//     fn apply<T: Write>(&self, dest: &mut T) -> std::io::Result<()> {
//         todo!()
//     }
//
//     fn from(&mut self, frame: &Frame) -> Result<Self, error::CommandError> {
//         let cmd_name = get_name(frame)?;
//         match frame {
//             Frame::Array(content) => {
//                 if cmd_name.to_ascii_uppercase() != "SET" {
//                     return Err(error::CommandError::MalformedPing);
//                 }
//                 if let Frame::Bulk(value) = &content[1] {
//                     self.key = value.to_string();
//                 }
//                 if let Frame::Bulk(value) = &content[2] {
//                     self.value = value.to_string();
//                 }
//                 Ok(())
//             }
//             _ => Err(error::CommandError::NotCmdFrame),
//         }
//     }
// }
//
// pub fn new() -> Set {
//     Set {
//         key: "".to_string(),
//         value: "".to_string(),
//     }
// }
