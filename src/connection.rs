use crate::cmd::{self, Command};
use crate::error::FrameError;
use crate::error::{CommandError, HandleCommandError};
use crate::frame::Frame;
use crate::{db, frame};
use bytes::{Buf, BytesMut};
use std::io;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
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
    // pub fn close(&self) -> io::Result<()> {
    //     self.conn.shutdown(Shutdown::Both)
    // }
    pub fn new(stream: TcpStream, state: Arc<db::State>) -> io::Result<Self> {
        // set write timeout on the stream as we won't be using async for now
        // stream.set_write_timeout(WRITE_TIMEOUT)?;
        // // stream_clone is a reference count for stream
        // let read_clone = stream.clo()?;
        // let write_clone = stream.try_clone()?;
        let (read_half, write_half) = stream.into_split();
        // let mut reader = BufReader::new(read_half);
        let writer = BufWriter::new(write_half);
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

            // Attempt to parse a frame. Continue if the received data is incomplete to parse a frame.
            match self.parse_frame() {
                Ok(fr) => return Ok(fr),
                Err(e) => {
                    if let FrameError::Incomplete = e {
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    fn parse_frame(&mut self) -> Result<Frame, FrameError> {
        let mut buf = io::Cursor::new(&self.buffer[..]);

        // now call frame decode on buf, try to decode from the data available
        // The internal buffer (self.buffer) is of type bytesMut which grow automatically if needed. So, advance
        // the cursor to consume the data processed. There is an exception, though; in case of incomplete frame, we
        // do not want to advance the cursor. We need to read more data to try to decode the frame again.
        match frame::decode(&mut buf) {
            Ok(fr) => {
                let pos = buf.position();
                self.buffer.advance(pos as usize);
                Ok(fr)
            }
            // Do not advance in case of incomplete Frame, we want to read more data to process it.
            Err(e) => {
                if let FrameError::Incomplete = e {
                } else {
                    let pos = buf.position();
                    self.buffer.advance(pos as usize);
                }
                Err(e)
            }
        }
    }

    /// write_frame writes a frame to the connection.
    pub async fn write_frame(&mut self, frame: &Frame) -> Result<(), io::Error> {
        let bytes = frame.encode();
        self.writer.write_all(bytes.as_slice()).await?;
        // We want the frame to be available immediately after being written so flush the buffer.
        self.writer.flush().await?;
        Ok(())
    }

    /// send_error sends an error response to the client
    pub async fn send_error(&mut self, err: &HandleCommandError) {
        let err_frame = Frame::Error(err.to_string());
        if let Err(e) = self.write_frame(&err_frame).await {
            error!("failed to send error to client: {}", e);
        }
        debug!("command processing failed: {}", err)
    }

    /// handle_command try to retrieve a command from a connection and process it.
    /// All command related errors are sent as response to the client, and the rest
    /// are returned to the caller for further processing.
    pub async fn handle_command(&mut self) -> Result<(), HandleCommandError> {
        // get frame fist
        let frame = self.read_frame().await?;
        debug!("received command frame: {:?}", frame);
        // now parse command from frame
        let command = Command::from_frame(frame)?;
        // run command
        command.apply(&mut self.writer, &self.state).await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_read_frame_with_real_network() {
    use tokio::net::{TcpListener, TcpStream};
    // Set up a Tcp listener
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Set up a separate task that accepts the connection and writes data to the stream
    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let _ = socket
                .write_all(b"*2\r\n$5\r\nhello\r\n:28\r\n+simple\r\n_\r\n#t\r\n")
                .await;
        }
    });

    // Create a TcpStream connected to the listener
    let stream = TcpStream::connect(addr).await.unwrap();

    // Set up your Connection
    let (read_half, write_half) = stream.into_split();
    let state =
        Arc::new(db::State::new(500, 8, Arc::new((Mutex::new(true), Condvar::new())), 90).unwrap());

    let mut conn = Connection {
        reader: read_half,
        writer: BufWriter::new(write_half),
        buffer: BytesMut::with_capacity(4 * 1024),
        state,
    };

    // Perform the test
    let got = conn.read_frame().await.unwrap();
    let mut want = Frame::array();
    want.push_back(Frame::Bulk("hello".to_string()))
        .expect("success");
    want.push_back(Frame::Integer(28)).expect("success");
    assert_eq!(got, want);
    // assert_eq!(got, Frame::Simple("Hello World".to_string()));

    // Add your assertions here...
}
