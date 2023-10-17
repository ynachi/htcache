//! Implementing Redis framing protocol
//! https://redis.io/docs/reference/protocol-spec/

use std::fmt::{Display, Formatter};
use std::io;

pub(crate) enum Frame {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(String),
    Array(Vec<Frame>),
    Null,
    Boolean(bool),
    Double(f64),
    // @TODO implement this later
    // Map(HashMap<String, Frame>),
    // Set(HashSet<Frame>),
}

impl Frame {
    /// Returns an empty array of frames
    pub(crate) fn array() -> Frame {
        Frame::Array(vec![])
    }

    /// Pushes a `Frame` to the back of the current frame.
    ///
    /// This function checks if the `self` is of type `Frame::Array(frames)` and
    /// pushes the supplied `frame` to it. It will return an `Ok(())` on successful insertion.
    ///
    /// # Arguments
    ///
    /// * `frame` - The `Frame` to be pushed into the `self`.
    ///
    /// # Errors
    ///
    /// If the `self` is not of type `Frame::Array(frames)`, an io::Error of `InvalidData` variant is returned
    /// with a message "can only push frames to vector variant frame".
    ///
    ///
    pub(crate) fn push_back(&mut self, frame: Frame) -> io::Result<()> {
        match self {
            Frame::Array(frames) => {
                frames.push(frame);
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "can only push frames to vector variant frame",
            )),
        }
    }
}

impl Display for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Frame::Simple(content) => write!(f, "+{}\r\n", content),
            Frame::Error(content) => write!(f, "-{}\r\n", content),
            Frame::Integer(content) => write!(f, ":{}\r\n", content),
            Frame::Bulk(content) => write!(f, "${}\r\n{}\r\n", content.len(), content),
            Frame::Array(content) => {
                let mut s = format!("*{}\r\n", content.len());
                for frame in content {
                    s.push_str(&frame.to_string());
                }
                write!(f, "{}", s)
            }
            Frame::Null => write!(f, "_\r\n"),
            Frame::Boolean(content) => {
                let encoded_bool = {
                    if *content {
                        "t"
                    } else {
                        "f"
                    }
                };
                write!(f, "#{}\r\n", encoded_bool)
            }
            Frame::Double(content) => write!(f, ",{}\r\n", content),
        }
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

        // Double
        assert_eq!(
            Frame::Double(1.23).to_string(),
            ",1.23\r\n",
            "Double format does not match"
        );
        assert_eq!(
            Frame::Double(-1.23).to_string(),
            ",-1.23\r\n",
            "Double format does not match"
        );
        assert_eq!(
            Frame::Double(10f64).to_string(),
            ",10\r\n",
            "Double format does not match"
        );
        //@TODO Do not support exponent for now. To fix
        // assert_eq!(Frame::Double(1.23e-42).to_string(), ",1.23e-42\r\n", "Double format does not match");
        // assert_eq!(Frame::Double(1.23E+4).to_string(), ",1.23E+4\r\n", "Double format does not match");

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
}
