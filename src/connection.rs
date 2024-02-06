use crate::cmd::{self, Command};
use crate::error::FrameError;
use crate::error::{CommandError, HandleCommandError};
use crate::frame::Frame;
use crate::{db, frame};
use bytes::BytesMut;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tracing::{debug, error};

/// The client should give up after a timeout attempt to write to the stream.
//@TODO: Set as application config
const WRITE_TIMEOUT: Option<Duration> = Some(Duration::new(0, 1000));

/// Represents a connection to the server. It contains the TCPStream returned by the connection
/// and a read buffer
pub struct Connection {
    reader: OwnedReadHalf,
    writer: BufWriter<OwnedWriteHalf>,
    //
    buffer: BytesMut,
    // a connection receives a reference of a Cache.
    state: Arc<db::State>,
}

impl Connection {
    pub fn close(&self) -> io::Result<()> {
        self.conn.shutdown(Shutdown::Both)
    }
    pub fn new(stream: TcpStream, state: Arc<db::State>) -> io::Result<Self> {
        // set write timeout on the stream as we won't be using async for now
        // stream.set_write_timeout(WRITE_TIMEOUT)?;
        // // stream_clone is a reference count for stream
        // let read_clone = stream.clo()?;
        // let write_clone = stream.try_clone()?;
        let (read_half, write_half) = stream.into_split();
        // let mut reader = BufReader::new(read_half);
        let mut writer = BufWriter::new(write_half);
        Ok(Self {
            reader: read_half,
            writer,
            buffer: BytesMut::with_capacity(4 * 1024),
            state,
        })
    }

    /// read_frame reads a frame from this connection.
    pub async fn read_frame(&mut self) -> Result<Frame, FrameError> {
        loop {
            let len = self.reader.read_buf(&mut self.buffer).await?;

            if len == 0 {
                return if self.buffer.is_empty() {
                    // Connection gracefully closed
                    Err(FrameError::EOF)
                } else {
                    // unintended connection reset
                    Err(FrameError::ConnectionReset)
                };
            }

            // attempt to parse frame
            // be sure to maintain the position of the cursor in the buffer
            // We cannot call a frame parsing function directly here because we need to keep
            // track of the state of the buffer
            self.parse_frame().expect("TODO: panic message");
        }
    }

    fn parse_frame(&mut self) -> Result<Frame, FrameError> {
        let mut buf = io::Cursor::new(&self.buffer[..]);

        // now call frame decode on buf, try to decode from the data available
        frame::decode(&mut buf)
    }

    /// write_frame writes a frame to the connection.
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
            error!("failed to send error to client: {}", e);
        }
        debug!("command processing failed: {}", err)
    }

    /// get_client_ip retrieves the IP of the client. It returns unknown_ip if it cannot get it.
    pub fn get_client_ip(&self) -> String {
        self.conn
            .peer_addr()
            .map_or("unknown_ip".to_string(), |v| v.to_string())
    }

    /// handle_command try to retrieve a command from a connection and process it.
    /// All command related errors are sent as response to the client, and the rest
    /// are returned to the caller for further processing.
    pub async fn handle_command(&mut self) -> Result<(), HandleCommandError> {
        // get frame fist
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
                    .apply(&mut self.writer, &self.state)
                    .unwrap_or_else(|err| {
                        // This error happens when things cannot be written to the connection,
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
