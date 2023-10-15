use crate::frame::Frame;
use std::io;
use std::io::{BufRead, Error, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// The client should give up after timeout attempt to write to the stream.
//@TODO: Set as application config
const WRITE_TIMEOUT: Option<Duration> = Some(Duration::new(0, 1000));

/// Represents a connection to the server. It contains the TCPStream returned by the connection
/// and a read buffer
struct Connection {
    stream: io::BufReader<TcpStream>,
    buffer: Vec<u8>,
}

impl Connection {
    pub(crate) fn new(stream: TcpStream) -> Self {
        stream
            .set_write_timeout(WRITE_TIMEOUT)
            .expect("set_write_timeout call failed");
        Self {
            stream: io::BufReader::new(stream),
            buffer: Vec::with_capacity(4 * 1024),
        }
    }

    /// Writes a frame to the TCP connection underlined stream
    fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        self.stream
            .get_mut()
            .write_all(frame.to_string().as_bytes())?;
        Ok(())
    }

    pub(crate) fn read_frame(&mut self) -> io::Result<Frame> {
        // read the first byte to identify the frame type
        let header_id = self.read_single_byte()?;

        let frame = match header_id {
            b'+' => self.read_integer_frame()?,
            _ => Frame::Null,
        };
        Ok(frame)
    }

    pub(crate) fn read_integer_frame(&mut self) -> io::Result<Frame> {
        // The next '\r\n' will be the boundary of integer to read
        let frame_val_str = self.read_until_crlf()?;
        // we successfully read the equivalent of an i64 in bytes, let's convert it
        let frame_val_i64: i64 = frame_val_str.parse().map_err(|_| self.atoi_error())?;
        // Return the successfully read Integer frame
        Ok(Frame::Integer(frame_val_i64))
    }

    /// Use to read a single byte. Typically used to read the frame header identifier or the from
    /// termination character.
    fn read_single_byte(&self) -> io::Result<u8> {
        let mut buffer = [0; 1];
        self.stream.get_ref().read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    /// The frames are delimited by crlf ("\r\n"). If \n cannot immediately be read after \r, we
    /// have an invalid frame. We return a string because it is easy to parse back to a valid frame.
    fn read_until_crlf(&mut self) -> io::Result<String> {
        self.reset_read_buffer();

        // Read until CR (0x0D) pr \r
        self.read_until(b'\r')?;

        // now read LF
        let bytes_read = self.read_until(b'\n')?;

        // the LF should immediately follow the previous CR
        if bytes_read != 1 {
            return Err(self.invalid_frame_error());
        }

        // now convert to string
        self.buffer_to_string()
    }

    fn reset_read_buffer(&mut self) {
        self.buffer.clear();
    }

    fn invalid_frame_error(&self) -> Error {
        Error::new(ErrorKind::InvalidData, "invalid frame")
    }

    fn atoi_error(&self) -> Error {
        Error::new(ErrorKind::InvalidData, "failed to parse String to i64")
    }

    fn read_until(&mut self, delimiter: u8) -> io::Result<usize> {
        self.stream.read_until(delimiter, &mut self.buffer)
    }

    fn buffer_to_string(&self) -> io::Result<String> {
        let result = String::from_utf8(self.buffer.clone())
            .map_err(|_| self.invalid_frame_error())?
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .to_string();

        Ok(result)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::net::{TcpListener, TcpStream};
//     use std::time::Duration;
//
//     // #[test]
//     // fn new_connection() {
//     //     // Start a TcpListener
//     //     let listener = TcpListener::bind("127.0.0.1:0").unwrap();
//     //     let server = listener.local_addr().unwrap();
//     //
//     //     // Create a TcpStream
//     //     let original_stream = TcpStream::connect(server).unwrap();
//     //
//     //     // Set the write timeout on the original stream for verification later
//     //     original_stream
//     //         .set_write_timeout(Some(Duration::new(0, 1000)))
//     //         .expect("set_write_timeout call failed");
//     //
//     //     // Create a new connection
//     //     let connection = Connection::new(original_stream.try_clone().unwrap());
//     //
//     //     // Check that the write timeout on the stream within the connection
//     //     // is the same as the one that was set on the original stream
//     //     let stream_in_connection_timeout = connection.stream.write_timeout().unwrap();
//     //     let original_stream_timeout = original_stream.write_timeout().unwrap();
//     //     assert_eq!(stream_in_connection_timeout, original_stream_timeout);
//     //
//     //     // check wrong timeout
//     //     let err = original_stream
//     //         .set_write_timeout(Some(Duration::new(0, 0)))
//     //         .unwrap_err();
//     //     assert_eq!(err.kind(), io::ErrorKind::InvalidInput)
//     // }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn test_read_until_crlf() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            write!(socket, "Hello\r\nWorld\r\nFail\rhello\nend\r\n").unwrap();
        });

        let stream = TcpStream::connect(server_addr).unwrap();
        let mut conn = Connection::new(stream);

        let resp1 = conn.read_until_crlf().unwrap();
        let resp2 = conn.read_until_crlf().unwrap();
        let resp3 = conn.read_until_crlf();
        let resp4 = conn.read_until_crlf().unwrap();

        assert_eq!("Hello", resp1);
        assert_eq!("World", resp2);
        assert!(
            resp3.is_err(),
            "finding only CR in the middle of a response is not allowed"
        );
        // assert_eq!("hello\nend", resp4);

        server.join().unwrap();
    }
}
