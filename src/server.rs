use crate::connection;
use crate::error::{CommandError, FrameError};
use crate::threadpool;
use std::io;
use std::net::TcpListener;

pub struct Server {
    thread_pool: threadpool::ThreadPool,
    listener: TcpListener,
}

/// new creates and start a new server.
/// `new` return a Result instead of the actual type.
/// It is required in this case because creating a new server requires
/// to prepare threads that it will use to process the requests.
/// And, creating threads are likely to fail for reasons related to the OS.
pub fn new(ip: String, port: u16, num_workers: usize) -> io::Result<Server> {
    let thread_pool = threadpool::ThreadPool::new(num_workers)?;
    let listener = TcpListener::bind((ip, port))?;
    Ok(Server {
        thread_pool,
        listener,
    })
}

impl Server {
    /// listen listens to incoming connections and process them.
    pub fn listen(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let conn = connection::Connection::new(stream);
                    match conn {
                        Ok(mut conn) => {
                            self.thread_pool
                                .execute(move || handle_connection(&mut conn));
                        }
                        Err(e) => {
                            eprintln!("failed to create connection object: {}", e)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("failed to establish connection: {}", e)
                }
            }
        }
    }
}

fn handle_connection(conn: &mut connection::Connection) {
    loop {
        match conn.handle_command() {
            Ok(_) => {}
            Err(FrameError::EOF) => break,
            Err(e) => {
                conn.send_error(&CommandError::FrameDecode(e));
            }
        };
    }
}
