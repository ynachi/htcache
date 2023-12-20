use crate::cmd::{get_name, ping, Command};
use crate::error::CommandError;
use crate::error::FrameError;
use crate::frame;
use crate::frame::Frame;
use std::io;
use std::io::{BufReader, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

/// The client should give up after timeout attempt to write to the stream.
//@TODO: Set as application config
const WRITE_TIMEOUT: Option<Duration> = Some(Duration::new(0, 1000));

/// Represents a connection to the server. It contains the TCPStream returned by the connection
/// and a read buffer
pub struct Connection {
    conn: TcpStream,
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

impl Connection {
    pub fn close(&self) -> io::Result<()> {
        self.conn.shutdown(Shutdown::Both)
    }
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        // set write timeout on the stream as we won't be using async for now
        stream.set_write_timeout(WRITE_TIMEOUT)?;
        // stream_clone is a reference count for stream
        let read_clone = stream.try_clone()?;
        let write_clone = stream.try_clone()?;
        Ok(Self {
            conn: stream,
            reader: BufReader::new(read_clone),
            writer: write_clone,
        })
    }

    /// read_frame reads a frame from this connection.
    pub fn read_frame(&mut self) -> Result<Frame, FrameError> {
        frame::decode(&mut self.reader)
    }

    /// read_frame reads a frame from this connection.
    pub fn write_frame(&mut self, frame: &Frame) -> Result<(), io::Error> {
        let bytes = frame.encode();
        self.writer.write_all(bytes.as_slice())?;
        self.writer.flush()?;
        Ok(())
    }

    /// send_error sends an error response to the client
    pub fn send_error(&mut self, err: &CommandError) {
        let err_frame = Frame::Error(err.to_string());
        if let Err(e) = self.write_frame(&err_frame) {
            eprintln!("failed to send error to client: {}", e);
        }
    }

    /// get_client_ip retrieves the IP of the client. It returns unknown_ip if it cannot get it.
    pub fn get_client_ip(&self) -> String {
        self.conn
            .peer_addr()
            .map_or("unknown_ip".to_string(), |v| v.to_string())
    }

    /// handle_command try to retrieve a command from a connection and process it.
    /// All command related errors are sent as response to the client and the rest
    /// are return to the caller for further processing.
    pub fn handle_command(&mut self) -> Result<(), FrameError> {
        //1. get frame fist
        let frame = self.read_frame()?;
        // 2. Get the command name
        let cmd_name = get_name(&frame);
        match cmd_name {
            Ok(cmd_name) => {
                // @TODO add db later
                self.apply_command(&cmd_name, &frame)
            }
            Err(e) => self.send_error(&e),
        }
        Ok(())
    }

    fn apply_command(&mut self, cmd_name: &str, frame: &Frame) {
        match cmd_name {
            "PING" => {
                let mut cmd = ping::new();
                cmd.from(frame).unwrap_or_else(|err| {
                    self.send_error(&err);
                });
                cmd.apply(&mut self.writer).unwrap_or_else(|err| {
                    eprintln!("error writing response to client: {}", err);
                });
            }
            _ => self.send_error(&CommandError::Unknown(cmd_name.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {}
