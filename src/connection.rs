use crate::cmd::{self, Command};
use crate::error::FrameError;
use crate::error::{CommandError, HandleCommandError};
use crate::frame::Frame;
use crate::{db, frame};
use std::io;
use std::io::{BufReader, BufWriter, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error};

/// The client should give up after timeout attempt to write to the stream.
//@TODO: Set as application config
const WRITE_TIMEOUT: Option<Duration> = Some(Duration::new(0, 1000));

/// Represents a connection to the server. It contains the TCPStream returned by the connection
/// and a read buffer
pub struct Connection {
    // conn is the raw tcp stream created when a connection is established.
    conn: TcpStream,
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
    // a connection receive a reference of a Cache.
    htcache: Arc<db::HTCache>,
}

impl Connection {
    pub fn close(&self) -> io::Result<()> {
        self.conn.shutdown(Shutdown::Both)
    }
    pub fn new(stream: TcpStream, htcache: Arc<db::HTCache>) -> io::Result<Self> {
        // set write timeout on the stream as we won't be using async for now
        stream.set_write_timeout(WRITE_TIMEOUT)?;
        // stream_clone is a reference count for stream
        let read_clone = stream.try_clone()?;
        let write_clone = stream.try_clone()?;
        Ok(Self {
            conn: stream,
            reader: BufReader::new(read_clone),
            writer: BufWriter::new(write_clone),
            htcache,
        })
    }

    /// read_frame reads a frame from this connection.
    pub fn read_frame(&mut self) -> Result<Frame, FrameError> {
        frame::decode(&mut self.reader)
    }

    /// write_frame writes a frame to the connection connection.
    pub fn write_frame(&mut self, frame: &Frame) -> Result<(), io::Error> {
        let bytes = frame.encode();
        self.writer.write_all(bytes.as_slice())?;
        self.writer.flush()?;
        Ok(())
    }

    /// send_error sends an error response to the client
    pub fn send_error(&mut self, err: &HandleCommandError) {
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
    pub fn handle_command(&mut self) -> Result<(), HandleCommandError> {
        //1. get frame fist
        let frame = self.read_frame()?;
        debug!("received command frame: {:?}", frame);
        let cmd_name = cmd::get_name(&frame)?;
        self.apply_command(&cmd_name, &frame);
        Ok(())
    }

    fn execute_command<Cmd>(&mut self, frame: &Frame)
    where
        Cmd: Command,
    {
        match Cmd::from(frame) {
            Ok(command) => {
                command
                    .apply(&mut self.writer, &self.htcache)
                    .unwrap_or_else(|err| {
                        error!(
                            error_message = err.to_string(),
                            "error writing response to client"
                        );
                    });
            }
            Err(err) => self.send_error(&HandleCommandError::Command(err)),
        }
    }

    fn apply_command(&mut self, cmd_name: &str, frame: &Frame) {
        match cmd_name {
            "PING" => self.execute_command::<cmd::Ping>(frame),
            "SET" => self.execute_command::<cmd::Set>(frame),
            "GET" => self.execute_command::<cmd::Get>(frame),
            "DEL" => self.execute_command::<cmd::Del>(frame),
            _ => self.send_error(&HandleCommandError::Command(CommandError::Unknown(
                cmd_name.to_string(),
            ))),
        }
    }
}
