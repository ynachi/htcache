use std::io;
use std::net::TcpStream;
use std::time::Duration;
use std::io::{BufRead, BufReader, Read, Write};
use crate::frame::Frame;

/// The client should give up after timeout attempt to write to the stream.
//@TODO: Set as application config
const WRITE_TIMEOUT: Option<Duration> = Some(Duration::new(0, 1000));

/// Represents a connection to the server. It contains the TCPStream returned by the connection
/// and a read buffer
struct Connection {
    stream: TcpStream,
    buffer: [u8; 4 * 1024],
    fist_byte_buffer: [u8; 1]
}

impl Connection {
    pub(crate) fn new(stream: TcpStream) -> Connection {
        stream.set_write_timeout(WRITE_TIMEOUT).expect("set_write_timeout call failed");
        Connection {
            stream,
            buffer: [0; 4* 1024],
            fist_byte_buffer: [0; 1]
        }
    }

    /// Writes a frame to the TCP connection underlined stream
    pub(crate) fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        self.stream.write_all(frame.to_string().as_bytes())?;
        Ok(())
    }

    pub(crate) fn read_frame(&mut self) -> io::Result<Frame> {
        // read the first byte to identify the frame type
        self.stream.read_exact(&mut self.fist_byte_buffer)?;

        let buff_reader = BufReader::new(&self.stream);

        match self.fist_byte_buffer[0] {
            b'+' => {
                buff_reader.read_until(b'\r\n')
            }
        }
        Ok(Frame::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    #[test]
    fn new_connection() {
        // Start a TcpListener
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let server = listener.local_addr().unwrap();

        // Create a TcpStream
        let original_stream = TcpStream::connect(server).unwrap();

        // Set the write timeout on the original stream for verification later
        original_stream.set_write_timeout(Some(Duration::new(0, 1000)))
            .expect("set_write_timeout call failed");

        // Create a new connection
        let connection = Connection::new(original_stream.try_clone().unwrap());

        // Check that the write timeout on the stream within the connection
        // is the same as the one that was set on the original stream
        let stream_in_connection_timeout = connection.stream.write_timeout().unwrap();
        let original_stream_timeout = original_stream.write_timeout().unwrap();
        assert_eq!(stream_in_connection_timeout, original_stream_timeout);

        // check wrong timeout
        let err = original_stream.set_write_timeout(Some(Duration::new(0, 0)))
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput)
    }
}
