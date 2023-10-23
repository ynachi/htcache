//! Implementing Redis framing protocol
//! https://redis.io/docs/reference/protocol-spec/

use crate::error::FrameError;
use std::fmt::{Display, Formatter};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Frame {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(String),
    Array(Vec<Frame>),
    Null,
    Boolean(bool),
}

impl Frame {
    /// Returns an empty array of frames
    fn array() -> Frame {
        Frame::Array(vec![])
    }

    /// Push frames to an a frame array variant
    pub(crate) fn push_back(&mut self, frame: Frame) -> Result<(), FrameError> {
        match self {
            Frame::Array(frames) => {
                frames.push(frame);
                Ok(())
            }
            _ => Err(FrameError::InvalidType(String::from(
                "can only push frames to vector variant frame",
            ))),
        }
    }
}

/// Turns a Frame into a slice of bytes, ready to be transferred though a network
pub(crate) fn encode(frame: &Frame) -> Vec<u8> {
    match frame {
        Frame::Simple(content) => {
            let mut bytes = vec![b'+'];
            bytes.extend(content.as_bytes());
            bytes.extend(b"\r\n");
            bytes
        }

        Frame::Error(content) => {
            let mut bytes = vec![b'-'];
            bytes.extend(content.as_bytes());
            bytes.extend(b"\r\n");
            bytes
        }

        Frame::Integer(content) => {
            let mut bytes = vec![b':'];
            bytes.extend(content.to_string().as_bytes());
            bytes.extend(b"\r\n");
            bytes
        }

        Frame::Bulk(content) => {
            let mut bytes = vec![b'$'];
            bytes.extend(content.len().to_string().as_bytes());
            bytes.extend(b"\r\n");
            bytes.extend(content.as_bytes());
            bytes.extend(b"\r\n");
            bytes
        }

        Frame::Boolean(content) => {
            let mut bytes = vec![b'#'];
            let shortened_bool = {
                if *content {
                    "t"
                } else {
                    "f"
                }
            };
            bytes.extend(shortened_bool.as_bytes());
            bytes.extend(b"\r\n");
            bytes
        }

        Frame::Null => {
            let mut bytes = vec![b'_'];
            bytes.extend(b"\r\n");
            bytes
        }

        Frame::Array(frames) => {
            let mut bytes = vec![b'*'];
            bytes.extend(frames.len().to_string().as_bytes());
            bytes.extend(b"\r\n");
            for f in frames {
                bytes.extend(encode(f));
            }
            bytes
        }
    }
}

/// buf could contain a series of frames read from a stream. Decode read a frame
/// from this buffer and return it and cursor position in the buffer to read the next
/// frame. If it encounters an invalid frame, it still return the next position
/// from which another frame can be read. This means invalid data will be lost
pub(crate) fn decode(buf: &[u8]) -> Result<(usize, Frame), FrameError> {
    match buf[0] {
        b':' => {
            // let int = read_int(position, buf)?;
            // next position = position + bytes read
            // return
            // to convert to int
            // read in a [u8;8], then convert
            let (next_position, int_value) = read_integer(buf)?;
            Ok((next_position, Frame::Integer(int_value)))
        }
        _ => unimplemented!(),
    }
}

/// Finds and return the position of the next CRLR in the buffer. Returns an invalid frame
/// error if it cannot be found. The position returned includes the CRLR chars.
fn crlr_position(buf: &[u8]) -> Result<usize, FrameError> {
    let mut lf_position =
        buf[..]
            .iter()
            .position(|&b| b == b'\n')
            .ok_or(FrameError::InvalidFrame(String::from(
                "frame does not contain any LF",
            )))?;

    if lf_position < 2 {
        return Err(FrameError::InvalidFrame(String::from(
            "buffer does not contain enough data",
        )));
    }

    // If CR not preceding LF, we are not at the boundary of a frame, so keep looking
    if buf[lf_position - 1] != b'\r' {
        // No CRLF found, search for next CRLF or reach the end of the buffer
        loop {
            let next_lf_position =
                // start searching from the previous position
                buf[lf_position+1..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .ok_or(FrameError::InvalidFrame(String::from(
                        "frame does not contain any CRLF",
                    )))?;
            // set new LF position
            lf_position += next_lf_position;

            if buf[lf_position - 1] == b'\r' {
                break;
            }
        }
    }

    Ok(lf_position)
}

/// Read an integer from this buffer. It is admitted that the frame identifier character ':' is
/// not included
fn read_integer(buf: &[u8]) -> Result<(usize, i64), FrameError> {
    let int_end = crlr_position(buf)? - 1;

    let int_str = String::from_utf8(buf[..int_end].to_vec())
        .map_err(|_| FrameError::InvalidFrame(String::from("failed to convert bytes to string")))?;

    let int_result: i64 = int_str
        .parse()
        .map_err(|_| FrameError::InvalidFrame(String::from("failed to parse string to integer")))?;

    // next position == int_end + 2
    Ok((int_end + 2, int_result))
}

impl Display for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let frame_as_bytes = encode(self);
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
    fn test_read_integer() {

        // read integer success
        let buf: &[_] = b"1234\r\n";
        let res = read_integer(buf).unwrap();
        assert_eq!(res, (6, 1234));

        // read a negative integer
        let buf: &[_] = b"-9876\r\n";
        let res = read_integer(buf).unwrap();
        assert_eq!(res, (7, -9876));

        // read integer with no crlr
        let buf: &[_] = b"1234";
        let res = read_integer(buf);
        assert!(res.is_err());

        // read invalid utf8
        let buf: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
        let res = read_integer(buf);
        assert!(res.is_err());

        // read non-numeric
        let buf: &[_] = b"abc\r\n";
        let res = read_integer(buf);
        assert!(res.is_err());

        // read empty integer
        let buf: &[_] = b"\r\n";
        let res = read_integer(buf);
        assert!(res.is_err());
    }
}
