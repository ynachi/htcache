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
    fn read_single_byte(&mut self) -> io::Result<u8> {
        let mut buffer = [0; 1];
        self.stream.read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    /// The frames are delimited by crlf ("\r\n"). If \n cannot immediately be read after \r, we
    /// have an invalid frame. We return a string because it is easy to parse back to a valid frame.
    fn read_until_crlf(&mut self) -> io::Result<String> {
        self.reset_read_buffer();

        // Read until CR (0x0D) pr \r
        self.read_until(b'\r')?;

        // now try read LF right after CR
        let try_lf = self.peek_single_byte()?;
        println!("============debug debug {} debug ========================", try_lf);

        // the LF should immediately follow the previous CR
        if try_lf != b'\n' {
            return Err(self.invalid_frame_error());
        }

        // You've found the right LF, remove it from the queue
        self.consume(1);

        // now convert to string
        self.buffer_to_string()
    }

    // This does not work, to fix
    fn peek_single_byte(&self) -> io::Result<u8> {
        let mut buffer = [0u8; 1];
        self.stream.get_ref().peek(&mut buffer)?;
        Ok(buffer[0])
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

    /// Consumes a number_of_bytes bytes from the connection. This is typically used with peak,
    /// when we need to check some condition before removing the data from the stream.
    fn consume(&mut self, number_of_bytes: usize) {
        self.stream.consume(number_of_bytes);
    }

    fn buffer_to_string(&self) -> io::Result<String> {
        let result = String::from_utf8(self.buffer.clone())
            .map_err(|_| self.invalid_frame_error())?
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .to_string();

        Ok(result)
    }
}

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
        assert_eq!("hello\nend", resp4);

        server.join().unwrap();
    }
}
