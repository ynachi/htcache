//! Implementing Redis framing protocol
//! https://redis.io/docs/reference/protocol-spec/

use crate::error::FrameError;
use bytes::Buf;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::Cursor;

const MAX_ITEM_SIZE: usize = 4 * 1024;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(String),
    Array(Vec<Frame>),
    Null,
    Boolean(bool),
    Map(BTreeMap<Frame, Frame>),
}

impl Ord for Frame {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Frame::Simple(a), Frame::Simple(b)) => a.cmp(b),
            (Frame::Error(a), Frame::Error(b)) => a.cmp(b),
            (Frame::Integer(a), Frame::Integer(b)) => a.cmp(b),
            (Frame::Bulk(a), Frame::Bulk(b)) => a.cmp(b),
            (Frame::Array(a), Frame::Array(b)) => a.cmp(b),
            (Frame::Map(a), Frame::Map(b)) => a.cmp(b),
            (Frame::Null, Frame::Null) => Ordering::Equal,
            (Frame::Boolean(a), Frame::Boolean(b)) => a.cmp(b),
            _ => panic!("Can't compare different Frame variants"),
        }
    }
}

impl PartialOrd for Frame {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Frame {
    /// array returns an empty array of frames
    pub fn array() -> Frame {
        Frame::Array(vec![])
    }

    /// map creates an empty map of frames
    pub fn map() -> Frame {
        Frame::Map(BTreeMap::new())
    }

    /// push_back push frames to an a frame array variant
    pub fn push_back(&mut self, frame: Frame) -> Result<(), FrameError> {
        match self {
            Frame::Array(frames) => {
                frames.push(frame);
                Ok(())
            }
            _ => Err(FrameError::InvalidType),
        }
    }

    /// add_map_frame add a frame to a Map of frames.
    pub fn add_map_frame(&mut self, key: Frame, value: Frame) -> Result<(), FrameError> {
        match self {
            Frame::Map(frames) => {
                frames.insert(key, value);
                Ok(())
            }
            _ => Err(FrameError::InvalidType),
        }
    }

    /// encode turns a Frame into a slice of bytes, ready to be transferred though a network
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Frame::Simple(content) => {
                let formatted_content = format!("+{}\r\n", content);
                formatted_content.as_bytes().to_vec()
            }

            Frame::Error(content) => {
                let formatted_content = format!("-{}\r\n", content);
                formatted_content.as_bytes().to_vec()
            }

            Frame::Integer(content) => {
                let formatted_content = format!(":{}\r\n", content);
                formatted_content.as_bytes().to_vec()
            }

            Frame::Bulk(content) => {
                let formatted_content = format!("${}\r\n{}\r\n", content.len(), content);
                formatted_content.as_bytes().to_vec()
            }

            Frame::Boolean(content) => {
                let shortened_bool = {
                    if *content {
                        "t"
                    } else {
                        "f"
                    }
                };
                let formatted_content = format!("#{}\r\n", shortened_bool);
                formatted_content.as_bytes().to_vec()
            }

            Frame::Null => {
                let formatted_content = "_\r\n".to_string();
                formatted_content.as_bytes().to_vec()
            }

            Frame::Array(frames) => {
                let mut bytes = vec![b'*'];
                bytes.extend(frames.len().to_string().as_bytes());
                bytes.extend(b"\r\n");
                for f in frames {
                    bytes.extend(f.encode());
                }
                bytes
            }

            Frame::Map(frames) => {
                let mut bytes = vec![b'%'];
                bytes.extend(frames.len().to_string().as_bytes());
                bytes.extend(b"\r\n");
                for (k, v) in frames {
                    bytes.extend(k.encode());
                    bytes.extend(v.encode())
                }
                bytes
            }
        }
    }

    /// write_to writes a frame to a writer
    pub fn write_to<T: io::Write>(&self, w: &mut T) -> Result<(), io::Error> {
        let bytes = self.encode();
        w.write_all(bytes.as_slice())?;
        w.flush()?;
        Ok(())
    }
}

/// Decode attempt to read a frame a buffer.
/// It first identifies the frame type and decodes it accordingly.
/// Keep in mind that the buffer might be partial and manage those cases.
/// Errors are generally malformed frames.
pub fn decode(buf: &mut Cursor<&[u8]>) -> Result<Frame, FrameError> {
    let tag = peek_byte(buf)?;
    match tag {
        // Simple String
        b'+' => {
            let content = read_simple_string(buf)?;
            Ok(Frame::Simple(content))
        }
        // Error
        b'-' => {
            let content = read_simple_string(buf)?;
            Ok(Frame::Error(content))
        }
        // Integer
        b':' => {
            let content_string = read_simple_string(buf)?;
            let content = content_string.parse()?;
            Ok(Frame::Integer(content))
        }
        // Bulk
        b'$' => {
            let content = read_bulk_string(buf)?;
            Ok(Frame::Bulk(content))
        }
        // Bool
        b'#' => {
            let content = read_simple_string(buf)?;
            if content == *"t" {
                Ok(Frame::Boolean(true))
            } else if content == *"f" {
                Ok(Frame::Boolean(false))
            } else {
                Err(FrameError::InvalidFrame)
            }
        }
        // Nil frame
        b'_' => {
            let content = read_simple_string(buf)?;
            if content == *"" {
                Ok(Frame::Null)
            } else {
                Err(FrameError::InvalidFrame)
            }
        }
        // Array
        b'*' => decode_array(buf),
        // Map
        b'%' => decode_map(buf),
        _ => Err(FrameError::InvalidType),
    }
}

fn peek_byte(buf: &mut Cursor<&[u8]>) -> Result<u8, FrameError> {
    if !buf.has_remaining() {
        return Err(FrameError::Incomplete);
    }
    Ok(buf.chunk()[0])
}

fn get_byte(buf: &mut Cursor<&[u8]>) -> Result<u8, FrameError> {
    if !buf.has_remaining() {
        return Err(FrameError::Incomplete);
    }
    Ok(buf.get_u8())
}

const LF: u8 = b'\n';
const CR: u8 = b'\r';

/// find_crlf_simple finds the position of the next crlf. It returns the index of the LF or an error is no CRLF can be found.
/// This function returns an error if there is any single CR or LF as it is not expected. The function advance the io::Cursor
/// at the appropriate position before returning to the caller in case of success or errors. All errors but FrameError::Incomplete
/// advance the cursor position.
fn find_crlf_simple(buf: &mut Cursor<&[u8]>) -> Result<usize, FrameError> {
    if buf.remaining() == 0 {
        return Err(FrameError::Incomplete);
    }
    let len = buf.get_ref().len();
    let mut index = 1;
    while index < len {
        let current_byte = buf.get_ref()[index];
        if current_byte == LF {
            if buf.get_ref()[index - 1] == CR {
                return handle_character_valid(buf, index, 0);
            } else {
                // a single LF is not allowed, so this is an error
                return handle_character_error(buf, index);
            }
        }
        if current_byte == CR {
            if index + 1 < len && buf.get_ref()[index + 1] == LF {
                return handle_character_valid(buf, index, 1);
            } else {
                // a single CR is not allowed, so this is an error
                return handle_character_error(buf, index);
            }
        }
        index += 1;
    }
    Err(FrameError::Incomplete)
}

fn handle_character_error(buf: &mut Cursor<&[u8]>, index: usize) -> Result<usize, FrameError> {
    buf.set_position(index as u64);
    Err(FrameError::InvalidFrame)
}

fn handle_character_valid(
    buf: &mut Cursor<&[u8]>,
    index: usize,
    offset: u64,
) -> Result<usize, FrameError> {
    buf.set_position(index as u64 + offset);
    Ok(index + 1)
}

/// find_crlf_bulk finds the position of the next crlf. It sets the position of CRLF, cursor at LF if found or an error if not.
/// Unlike .find_crlf_simple, this bytes can contain CR or LF in the middle. The io cursor is not modified by this function.
///
fn find_crlf_bulk(buf: &mut Cursor<&[u8]>) -> Result<usize, FrameError> {
    if buf.remaining() == 0 {
        return Err(FrameError::Incomplete);
    }

    let len = buf.get_ref().len();
    let mut index = 1;

    while index < len {
        let current_byte = buf.get_ref()[index];

        if current_byte == b'\n' && buf.get_ref()[index - 1] == b'\r' {
            buf.set_position(index as u64);
            return Ok(index);
        }

        index += 1;
    }

    Err(FrameError::Incomplete)
}

/// string_from reads a simple string from a reader.
fn read_simple_string(buf: &mut Cursor<&[u8]>) -> Result<String, FrameError> {
    let pos = find_crlf_simple(buf)?;
    let content = String::from_utf8(buf.get_ref()[..=pos - 2].to_vec())?;
    Ok(content)
}

/// read_bulk_string_from_reader reads a bulk string from a reader.
fn read_bulk_string(buf: &mut Cursor<&[u8]>) -> Result<String, FrameError> {
    let pos = find_crlf_bulk(buf)?;
    let content = String::from_utf8(buf.get_ref()[..=pos - 2].to_vec())?;
    Ok(content)
}

/// decode_array decodes a frame Array from a reader.
/// The tag identifying the frame is considered to be already read.
fn decode_array(buf: &mut Cursor<&[u8]>) -> Result<Frame, FrameError> {
    // Read the length first
    let array_length = read_simple_string(buf)?;
    let array_length = array_length.parse()?;

    let mut arr = Frame::array();

    for _ in 0..array_length {
        let fr = decode(buf)?;
        arr.push_back(fr)?;
    }

    Ok(arr)
}

/// decode_map decodes a frame map from a reader.
/// The tag identifying the frame is considered to be already read.
fn decode_map(buf: &mut Cursor<&[u8]>) -> Result<Frame, FrameError> {
    // Read the length first
    let map_length = read_simple_string(buf)?;
    let map_length = map_length.parse()?;

    let mut map = Frame::map();

    for _ in 0..map_length {
        let key = decode(buf)?;
        let value = decode(buf)?;
        map.add_map_frame(key, value)?;
    }

    Ok(map)
}

impl Display for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let frame_as_bytes = self.encode();
        // we can use unwrapping because bytes converted from frame will always
        // have valid utf8 chars
        write!(
            f,
            "{}",
            String::from_utf8(frame_as_bytes).unwrap_or("invalid frame".to_string())
        )
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::io;
//     use tokio_test::io::Builder;
//
//     #[test]
//     fn test_frame_fmt() {
//         // Simple String
//         assert_eq!(
//             Frame::Simple("OK".to_string()).to_string(),
//             "+OK\r\n",
//             "Simple string format does not match"
//         );
//
//         // Error
//         assert_eq!(
//             Frame::Error("Error".to_string()).to_string(),
//             "-Error\r\n",
//             "Error format does not match"
//         );
//
//         // Integer
//         assert_eq!(
//             Frame::Integer(128).to_string(),
//             ":128\r\n",
//             "Integer format does not match"
//         );
//
//         // Bulk string
//         assert_eq!(
//             Frame::Bulk("hello".to_string()).to_string(),
//             "$5\r\nhello\r\n",
//             "Bulk format does not match"
//         );
//         assert_eq!(
//             Frame::Bulk("".to_string()).to_string(),
//             "$0\r\n\r\n",
//             "Bulk format does not match"
//         );
//
//         // Bool
//         assert_eq!(
//             Frame::Boolean(true).to_string(),
//             "#t\r\n",
//             "Bool format does not match"
//         );
//         assert_eq!(
//             Frame::Boolean(false).to_string(),
//             "#f\r\n",
//             "Bool format does not match"
//         );
//
//         // Null
//         assert_eq!(
//             Frame::Null.to_string(),
//             "_\r\n",
//             "Double format does not match"
//         );
//
//         // Array
//         let empty_array = Frame::array(); // Beware this is the Frame::Array constructor and not Frame::Array itself
//         assert_eq!(
//             empty_array.to_string(),
//             "*0\r\n",
//             "Empty Array format does not match"
//         );
//
//         let mut array_of_bulk = Frame::array();
//         array_of_bulk
//             .push_back(Frame::Bulk("hello".to_string()))
//             .unwrap();
//         array_of_bulk
//             .push_back(Frame::Bulk("world".to_string()))
//             .unwrap();
//         assert_eq!(
//             array_of_bulk.to_string(),
//             "*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n",
//             "Array of bulk format does not match"
//         );
//
//         let mut array_of_ints = Frame::array();
//         array_of_ints.push_back(Frame::Integer(1)).unwrap();
//         array_of_ints.push_back(Frame::Integer(2)).unwrap();
//         array_of_ints.push_back(Frame::Integer(3)).unwrap();
//         assert_eq!(
//             array_of_ints.to_string(),
//             "*3\r\n:1\r\n:2\r\n:3\r\n",
//             "Array of integer format does not match"
//         );
//
//         let mut array_of_mixed = Frame::array();
//         array_of_mixed.push_back(Frame::Integer(1)).unwrap();
//         array_of_mixed.push_back(Frame::Integer(2)).unwrap();
//         array_of_mixed.push_back(Frame::Integer(3)).unwrap();
//         array_of_mixed.push_back(Frame::Integer(4)).unwrap();
//         array_of_mixed
//             .push_back(Frame::Bulk("hello".to_string()))
//             .unwrap();
//         assert_eq!(
//             array_of_mixed.to_string(),
//             "*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$5\r\nhello\r\n",
//             "Array of mixed types format does not match"
//         );
//     }
//
//     #[test]
//     fn test_read_until_crlf_simple() {
//         // @TODO: Here is how to test tokio stream without opening network connection
//         let mut rd = Builder::new()
//             .read(b"Hello\r\nHello World\r\nWord\nHello")
//             .build();
//         // can read word word
//         let got = read_until_crlf_simple(&mut rd).unwrap();
//         assert_eq!(got, b"Hello");
//         // can read a string of multiple words
//         let got = read_until_crlf_simple(&mut rd).unwrap();
//         assert_eq!(got, b"Hello World");
//         // CR in the middle
//         let got = read_until_crlf_simple(&mut rd);
//         match got {
//             Err(FrameError::InvalidFrame) => {}
//             _ => panic!("Expected an Err FrameError"),
//         }
//         // Should not contain CR
//         let cursor = io::Cursor::new(b"Hello\rWorld\r\n");
//         let mut rd = BufStream::new(cursor);
//         let got = read_until_crlf_simple(&mut rd);
//         match got {
//             Err(FrameError::InvalidFrame) => {}
//             _ => panic!("Expected an Err FrameError"),
//         }
//     }
//
//     #[test]
//     fn test_read_until_crlf_bulk() {
//         let cursor = io::Cursor::new(b"5\r\nHello\r\n11\r\nHello World\r\nWord\nHello");
//         let mut rd = BufStream::new(cursor);
//         // can read word word
//         let got = read_until_crlf_bulk(&mut rd).unwrap();
//         assert_eq!(got, b"Hello");
//         // can read a string of multiple words
//         let got = read_until_crlf_bulk(&mut rd).unwrap();
//         assert_eq!(got, b"Hello World");
//         // CR in the middle
//         let got = read_until_crlf_bulk(&mut rd);
//         match got {
//             Err(FrameError::InvalidFrame) => {}
//             _ => panic!("Expected an Err FrameError"),
//         }
//         // Should not contain CR
//         let cursor = io::Cursor::new(b"11\r\nHello\rWorld\r\n");
//         let mut rd = BufStream::new(cursor);
//         let got = read_until_crlf_bulk(&mut rd).unwrap();
//         assert_eq!(got, b"Hello\rWorld");
//     }
//
//     #[test]
//     fn test_decode_simple_string() {
//         // test decode simple string
//         let cursor = io::Cursor::new(b"+Hello\r\n+Hello World\r\n+Word\n+\r\nHello");
//         let mut rd = BufStream::new(cursor);
//         // can read word word
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Simple("Hello".to_string()));
//         // can read a string of multiple words
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Simple("Hello World".to_string()));
//         // LF in the middle is not allowed
//         let got = decode(&mut rd);
//         match got {
//             Err(FrameError::InvalidFrame) => {}
//             _ => panic!("Expected an Err FrameError"),
//         }
//         // frame can be empty
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Simple("".to_string()));
//     }
//
//     #[test]
//     fn test_decode_error() {
//         let cursor = io::Cursor::new(b"-Hello\r\n-Hello World\r\n-Word\n-\r\nHello");
//         let mut rd = BufStream::new(cursor);
//         // can read word word
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Error("Hello".to_string()));
//         // can read a string of multiple words
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Error("Hello World".to_string()));
//         // LF in the middle is not allowed
//         let got = decode(&mut rd);
//         match got {
//             Err(FrameError::InvalidFrame) => {}
//             _ => panic!("Expected an Err FrameError"),
//         }
//         // frame can be empty
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Error("".to_string()));
//     }
//
//     #[test]
//     fn test_decode_integer() {
//         let cursor = io::Cursor::new(b":25\r\n:-25\r\n:0\r\n:notnumber\r\n:33");
//         let mut rd = BufStream::new(cursor);
//         // can read positive number
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Integer(25));
//         // can read negative number
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Integer(-25));
//         // Can read 0
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Integer(0));
//         // non number should fail
//         let got = decode(&mut rd);
//         match got {
//             Err(FrameError::IntFromUTF8(_)) => {}
//             _ => panic!("Expected an Err FrameError"),
//         }
//         // Should be properly terminated
//         let got = decode(&mut rd);
//         match got {
//             Err(FrameError::InvalidFrame) => {}
//             _ => panic!("Expected an Err FrameError"),
//         }
//     }
//
//     #[test]
//     fn test_decode_bulk() {
//         let cursor = io::Cursor::new(b"$5\r\nHello\r\n$5\r\nWrong Size\r\n");
//         let mut rd = BufStream::new(cursor);
//         // can read word word
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Bulk("Hello".to_string()));
//         // Size does not match content
//         let got = decode(&mut rd);
//         match got {
//             Err(FrameError::InvalidFrame) => {}
//             _ => panic!("Expected an Err FrameError"),
//         }
//     }
//
//     #[test]
//     fn test_decode_bool() {
//         let cursor = io::Cursor::new(b"#t\r\n#f\r\n#5\r\nWrong\r\n");
//         let mut rd = BufStream::new(cursor);
//         // can get true
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Boolean(true));
//         // can get true
//         let got = decode(&mut rd).unwrap();
//         assert_eq!(got, Frame::Boolean(false));
//         // This is not a bool
//         let got = decode(&mut rd);
//         match got {
//             Err(FrameError::InvalidFrame) => {}
//             _ => panic!("Expected an Err FrameError"),
//         }
//     }
//
//     #[test]
//     fn test_decode_array() {
//         let cursor = io::Cursor::new(b"*2\r\n$5\r\nhello\r\n:28\r\n+simple\r\n_\r\n#t\r\n");
//         let mut rd = BufStream::new(cursor);
//         // good array
//         let got = decode(&mut rd).unwrap();
//         let mut want = Frame::array();
//         want.push_back(Frame::Bulk("hello".to_string()))
//             .expect("success");
//         want.push_back(Frame::Integer(28)).expect("success");
//         assert_eq!(got, want);
//
//         // bad array
//         let cursor = io::Cursor::new(b"*2\r\n$5\r\nhello1\r\n:28\r\n+simple\r\n_\r\n#t\r\n");
//         let mut rd = BufStream::new(cursor);
//         // Size does not match content
//         let got = decode(&mut rd);
//         match got {
//             Err(FrameError::InvalidFrame) => {}
//             _ => panic!("Expected an Err FrameError"),
//         }
//     }
// }
