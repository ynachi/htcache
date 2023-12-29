//! Implementing Redis framing protocol
//! https://redis.io/docs/reference/protocol-spec/

use crate::error::FrameError;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::{BufRead, BufReader, Read};

const MAX_ITEM_SIZE: usize = 4 * 1024;

#[derive(Debug, Eq, PartialEq)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(String),
    Array(Vec<Frame>),
    Null,
    Boolean(bool),
}

impl Frame {
    /// array returns an empty array of frames
    fn array() -> Frame {
        Frame::Array(vec![])
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

/// decode attempt to read a frame a reader.
/// It first identify the frame type and decode it accordingly.
/// Bytes read are lost if an error occurs.
/// Errors are generally malformed frames.
pub fn decode<T: Read>(rd: &mut BufReader<T>) -> Result<Frame, FrameError> {
    let tag = read_single_byte(rd)?;
    match tag {
        // Simple String
        b'+' => {
            let content = read_simple_string(rd)?;
            Ok(Frame::Simple(content))
        }
        // Error
        b'-' => {
            let content = read_simple_string(rd)?;
            Ok(Frame::Error(content))
        }
        // Integer
        b':' => {
            let content_string = read_simple_string(rd)?;
            let content = content_string.parse()?;
            Ok(Frame::Integer(content))
        }
        // Bulk
        b'$' => {
            let content = read_bulk_string(rd)?;
            Ok(Frame::Bulk(content))
        }
        // Bool
        b'#' => {
            let content = read_simple_string(rd)?;
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
            let content = read_simple_string(rd)?;
            if content == *"" {
                Ok(Frame::Null)
            } else {
                Err(FrameError::InvalidFrame)
            }
        }
        // Array
        b'*' => decode_array(rd),
        _ => Err(FrameError::InvalidType),
    }
}

/// read_single_byte try to read a single byte from the reader.
/// It is typically used to get the frame tag.
fn read_single_byte(rd: &mut BufReader<impl Read>) -> Result<u8, FrameError> {
    let mut buffer = [0; 1];
    let size = rd.read(&mut buffer)?;
    if size == 0 {
        return Err(FrameError::EOF);
    }
    Ok(buffer[0])
}

/// string_from reads a simple string from a reader.
fn read_simple_string(rd: &mut BufReader<impl Read>) -> Result<String, FrameError> {
    let bytes = read_until_crlf_simple(rd)?;
    let content = String::from_utf8(bytes)?;
    Ok(content)
}

// read_until_crlf_simple reads a simple vector of u8 from a reader until CRLF.
// A simple vector is a vector which does not contain any CR, LF or CRLF in the middle.
// So it is considered to be an error to find such in the middle.
fn read_until_crlf_simple(rd: &mut BufReader<impl Read>) -> Result<Vec<u8>, FrameError> {
    let mut buff: Vec<u8> = Vec::with_capacity(MAX_ITEM_SIZE);
    let num_bytes = rd.read_until(b'\n', &mut buff)?;

    validate_simple_buff(&buff, num_bytes)?;

    buff.truncate(buff.len() - 2);

    Ok(buff)
}

fn validate_simple_buff(buff: &Vec<u8>, num_bytes: usize) -> Result<(), FrameError> {
    // This is EOF, returns a different error for it
    if num_bytes == 0 {
        return Err(FrameError::EOF);
    }
    if num_bytes < 2 || buff[buff.len() - 2] != b'\r' || buff[..buff.len() - 2].contains(&b'\r') {
        return Err(FrameError::InvalidFrame);
    }
    Ok(())
}

/// read_bulk_string_from_reader reads a bulk string from a reader.
fn read_bulk_string(rd: &mut BufReader<impl Read>) -> Result<String, FrameError> {
    let bytes = read_until_crlf_bulk(rd)?;
    let content = String::from_utf8(bytes)?;
    Ok(content)
}

// read_until_crlf_bulk reads a non simple string from the reader.
// This type of strings can contain CR or LF in them.
fn read_until_crlf_bulk(rd: &mut BufReader<impl Read>) -> Result<Vec<u8>, FrameError> {
    // Read the size first
    let frame_size = read_simple_string(rd)?;
    let frame_size = frame_size.parse()?;

    // now read the number of relevant bytes + CRLF
    // reminder: bulk frame is like $<length>\r\n<data>\r\n where length == len(data)
    let mut buff = vec![0; frame_size + 2];
    let read_size = rd.read(&mut buff)?;
    validate_bulk_buff(&buff, read_size, frame_size)?;

    buff.truncate(buff.len() - 2);

    Ok(buff)
}

/// validate_bulk_buff checks if a bulk string read from a buffer is valid
fn validate_bulk_buff(
    buff: &Vec<u8>,
    read_size: usize,
    frame_size: usize,
) -> Result<(), FrameError> {
    // This is EOF, returns a different error for it
    if read_size == 0 {
        return Err(FrameError::EOF);
    }
    if read_size != frame_size + 2 || buff[buff.len() - 2] != b'\r' {
        return Err(FrameError::InvalidFrame);
    }
    Ok(())
}

/// decode_array decodes a frame Array from a reader.
/// The tag identifying the frame is considered to be already read.
fn decode_array(rd: &mut BufReader<impl Read>) -> Result<Frame, FrameError> {
    // Read the length first
    let array_length = read_simple_string(rd)?;
    let array_length = array_length.parse()?;

    let mut arr = Frame::array();

    for _ in 0..array_length {
        let fr = decode(rd)?;
        arr.push_back(fr)?;
    }

    Ok(arr)
}

impl Display for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let frame_as_bytes = self.encode();
        // we can use unwrap because bytes converted from frame will always
        // has valid utf8 chars
        write!(
            f,
            "{}",
            String::from_utf8(frame_as_bytes).unwrap_or("invalid frame".to_string())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_frame_fmt() {
        // Simple String
        assert_eq!(
            Frame::Simple("OK".to_string()).to_string(),
            "+OK\r\n",
            "Simple string format does not match"
        );

        // Error
        assert_eq!(
            Frame::Error("Error".to_string()).to_string(),
            "-Error\r\n",
            "Error format does not match"
        );

        // Integer
        assert_eq!(
            Frame::Integer(128).to_string(),
            ":128\r\n",
            "Integer format does not match"
        );

        // Bulk string
        assert_eq!(
            Frame::Bulk("hello".to_string()).to_string(),
            "$5\r\nhello\r\n",
            "Bulk format does not match"
        );
        assert_eq!(
            Frame::Bulk("".to_string()).to_string(),
            "$0\r\n\r\n",
            "Bulk format does not match"
        );

        // Bool
        assert_eq!(
            Frame::Boolean(true).to_string(),
            "#t\r\n",
            "Bool format does not match"
        );
        assert_eq!(
            Frame::Boolean(false).to_string(),
            "#f\r\n",
            "Bool format does not match"
        );

        // Null
        assert_eq!(
            Frame::Null.to_string(),
            "_\r\n",
            "Double format does not match"
        );

        // Array
        let empty_array = Frame::array(); // Beware this is the Frame::Array constructor and not Frame::Array itself
        assert_eq!(
            empty_array.to_string(),
            "*0\r\n",
            "Empty Array format does not match"
        );

        let mut array_of_bulk = Frame::array();
        array_of_bulk
            .push_back(Frame::Bulk("hello".to_string()))
            .unwrap();
        array_of_bulk
            .push_back(Frame::Bulk("world".to_string()))
            .unwrap();
        assert_eq!(
            array_of_bulk.to_string(),
            "*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n",
            "Array of bulk format does not match"
        );

        let mut array_of_ints = Frame::array();
        array_of_ints.push_back(Frame::Integer(1)).unwrap();
        array_of_ints.push_back(Frame::Integer(2)).unwrap();
        array_of_ints.push_back(Frame::Integer(3)).unwrap();
        assert_eq!(
            array_of_ints.to_string(),
            "*3\r\n:1\r\n:2\r\n:3\r\n",
            "Array of integer format does not match"
        );

        let mut array_of_mixed = Frame::array();
        array_of_mixed.push_back(Frame::Integer(1)).unwrap();
        array_of_mixed.push_back(Frame::Integer(2)).unwrap();
        array_of_mixed.push_back(Frame::Integer(3)).unwrap();
        array_of_mixed.push_back(Frame::Integer(4)).unwrap();
        array_of_mixed
            .push_back(Frame::Bulk("hello".to_string()))
            .unwrap();
        assert_eq!(
            array_of_mixed.to_string(),
            "*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$5\r\nhello\r\n",
            "Array of mixed types format does not match"
        );
    }

    #[test]
    fn test_read_until_crlf_simple() {
        let cursor = io::Cursor::new(b"Hello\r\nHello World\r\nWord\nHello");
        let mut rd = BufReader::new(cursor);
        // can read word word
        let got = read_until_crlf_simple(&mut rd).unwrap();
        assert_eq!(got, b"Hello");
        // can read a string of multiple words
        let got = read_until_crlf_simple(&mut rd).unwrap();
        assert_eq!(got, b"Hello World");
        // CR in the middle
        let got = read_until_crlf_simple(&mut rd);
        match got {
            Err(FrameError::InvalidFrame) => {}
            _ => panic!("Expected an Err FrameError"),
        }
        // Should not contain CR
        let cursor = io::Cursor::new(b"Hello\rWorld\r\n");
        let mut rd = BufReader::new(cursor);
        let got = read_until_crlf_simple(&mut rd);
        match got {
            Err(FrameError::InvalidFrame) => {}
            _ => panic!("Expected an Err FrameError"),
        }
    }

    #[test]
    fn test_read_until_crlf_bulk() {
        let cursor = io::Cursor::new(b"5\r\nHello\r\n11\r\nHello World\r\nWord\nHello");
        let mut rd = BufReader::new(cursor);
        // can read word word
        let got = read_until_crlf_bulk(&mut rd).unwrap();
        assert_eq!(got, b"Hello");
        // can read a string of multiple words
        let got = read_until_crlf_bulk(&mut rd).unwrap();
        assert_eq!(got, b"Hello World");
        // CR in the middle
        let got = read_until_crlf_bulk(&mut rd);
        match got {
            Err(FrameError::InvalidFrame) => {}
            _ => panic!("Expected an Err FrameError"),
        }
        // Should not contain CR
        let cursor = io::Cursor::new(b"11\r\nHello\rWorld\r\n");
        let mut rd = BufReader::new(cursor);
        let got = read_until_crlf_bulk(&mut rd).unwrap();
        assert_eq!(got, b"Hello\rWorld");
    }

    #[test]
    fn test_decode_simple_string() {
        // test decode simple string
        let cursor = io::Cursor::new(b"+Hello\r\n+Hello World\r\n+Word\n+\r\nHello");
        let mut rd = BufReader::new(cursor);
        // can read word word
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Simple("Hello".to_string()));
        // can read a string of multiple words
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Simple("Hello World".to_string()));
        // LF in the middle is not allowed
        let got = decode(&mut rd);
        match got {
            Err(FrameError::InvalidFrame) => {}
            _ => panic!("Expected an Err FrameError"),
        }
        // frame can be empty
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Simple("".to_string()));
    }

    #[test]
    fn test_decode_error() {
        let cursor = io::Cursor::new(b"-Hello\r\n-Hello World\r\n-Word\n-\r\nHello");
        let mut rd = BufReader::new(cursor);
        // can read word word
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Error("Hello".to_string()));
        // can read a string of multiple words
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Error("Hello World".to_string()));
        // LF in the middle is not allowed
        let got = decode(&mut rd);
        match got {
            Err(FrameError::InvalidFrame) => {}
            _ => panic!("Expected an Err FrameError"),
        }
        // frame can be empty
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Error("".to_string()));
    }

    #[test]
    fn test_decode_integer() {
        let cursor = io::Cursor::new(b":25\r\n:-25\r\n:0\r\n:notnumber\r\n:33");
        let mut rd = BufReader::new(cursor);
        // can read positive number
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Integer(25));
        // can read negative number
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Integer(-25));
        // Can read 0
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Integer(0));
        // non number should fail
        let got = decode(&mut rd);
        match got {
            Err(FrameError::InvalidFrame) => {}
            _ => panic!("Expected an Err FrameError"),
        }
        // Should be properly terminated
        let got = decode(&mut rd);
        match got {
            Err(FrameError::InvalidFrame) => {}
            _ => panic!("Expected an Err FrameError"),
        }
    }

    #[test]
    fn test_decode_bulk() {
        let cursor = io::Cursor::new(b"$5\r\nHello\r\n$5\r\nWrong Size\r\n");
        let mut rd = BufReader::new(cursor);
        // can read word word
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Bulk("Hello".to_string()));
        // Size does not match content
        let got = decode(&mut rd);
        match got {
            Err(FrameError::InvalidFrame) => {}
            _ => panic!("Expected an Err FrameError"),
        }
    }

    #[test]
    fn test_decode_bool() {
        let cursor = io::Cursor::new(b"#t\r\n#f\r\n#5\r\nWrong\r\n");
        let mut rd = BufReader::new(cursor);
        // can get true
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Boolean(true));
        // can get true
        let got = decode(&mut rd).unwrap();
        assert_eq!(got, Frame::Boolean(false));
        // This is not a bool
        let got = decode(&mut rd);
        match got {
            Err(FrameError::InvalidFrame) => {}
            _ => panic!("Expected an Err FrameError"),
        }
    }

    #[test]
    fn test_decode_array() {
        let cursor = io::Cursor::new(b"*2\r\n$5\r\nhello\r\n:28\r\n+simple\r\n_\r\n#t\r\n");
        let mut rd = BufReader::new(cursor);
        // good array
        let got = decode(&mut rd).unwrap();
        let mut want = Frame::array();
        want.push_back(Frame::Bulk("hello".to_string()))
            .expect("success");
        want.push_back(Frame::Integer(28)).expect("success");
        assert_eq!(got, want);

        // bad array
        let cursor = io::Cursor::new(b"*2\r\n$5\r\nhello1\r\n:28\r\n+simple\r\n_\r\n#t\r\n");
        let mut rd = BufReader::new(cursor);
        // Size does not match content
        let got = decode(&mut rd);
        match got {
            Err(FrameError::InvalidFrame) => {}
            _ => panic!("Expected an Err FrameError"),
        }
    }
}
