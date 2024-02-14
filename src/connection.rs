use crate::cmd::{self, parse_frame, Command};
use crate::error::{CommandError, HandleCommandError};
use crate::frame::Frame;
use crate::{db, frame};
use std::io;
use std::io::{BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::sync::Arc;
use tracing::{debug, error};

/// Connection struct contains the TCP Stream derived from an established connection. Both reader
/// and writer share the same underline stream. State is a shared reference of the Cache database
pub struct Connection {
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
    state: Arc<db::State>,
}

impl Connection {
    // pub fn close(&self) -> io::Result<()> {
    //     self.conn.shutdown(Shutdown::Both)
    // }
    pub fn new(stream: TcpStream, state: Arc<db::State>) -> io::Result<Self> {
        let stream_clone = stream.try_clone()?;
        // let mut reader = BufReader::new(read_half);
        let writer = BufWriter::new(stream_clone);
        let reader = BufReader::new(stream);
        Ok(Self {
            reader,
            writer,
            state,
        })
    }

    pub fn close(&self) -> io::Result<()> {
        // Both reader and writer are linked to the same tcp stream so closing on only one is ok.
        self.reader.get_ref().shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }

    /// write_frame writes a frame to the connection.
    pub fn write_frame(&mut self, frame: &Frame) -> Result<(), io::Error> {
        let bytes = frame.encode();
        self.writer.write_all(bytes.as_slice())?;
        // We want the frame to be available immediately after being writen so flush the buffer.
        self.writer.flush()?;
        Ok(())
    }

    /// send_error converts an error to a Frame Error and send it back to the client.
    pub fn send_error(&mut self, err: &HandleCommandError) {
        let err_frame = Frame::Error(err.to_string());
        if let Err(e) = self.write_frame(&err_frame) {
            error!("failed to send error to client: {}", e);
        }
        debug!("command processing failed: {}", err)
    }

    /// handle_command try to retrieve a command from a connection and process it.
    /// All command related errors are sent as response to the client, and the rest
    /// are returned to the caller for further processing.
    pub fn handle_command(&mut self) -> Result<(), HandleCommandError> {
        // get frame fist
        let frame = frame::decode(&mut self.reader)?;
        debug!("received command frame: {:?}", frame);
        // parse frame
        let (cmd_name, frames) = parse_frame(frame)?;
        self.apply_command(&cmd_name, frames);
        Ok(())
    }

    fn execute_command<Cmd>(&mut self, frames: Vec<Frame>)
    where
        Cmd: Command,
    {
        match Cmd::from(frames) {
            Ok(command) => {
                command
                    .apply(&mut self.writer, &self.state)
                    .unwrap_or_else(|err| {
                        // This error happens when the data cannot be written to the connection,
                        // So it is not useful to try to send it to the client over the connection.
                        error!(
                            error_message = err.to_string(),
                            "error writing response to client"
                        );
                    });
            }
            Err(err) => self.send_error(&HandleCommandError::Command(err)),
        }
    }

    fn apply_command(&mut self, cmd_name: &str, frames: Vec<Frame>) {
        match cmd_name {
            "PING" => self.execute_command::<cmd::Ping>(frames),
            "SET" => self.execute_command::<cmd::Set>(frames),
            "GET" => self.execute_command::<cmd::Get>(frames),
            "DEL" => self.execute_command::<cmd::Del>(frames),
            _ => self.send_error(&HandleCommandError::Command(CommandError::Unknown(
                cmd_name.to_string(),
            ))),
        }
    }
}
