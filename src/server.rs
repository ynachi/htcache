use crate::error::FrameError;
use crate::frame::Frame;
use crate::{cmd, error, threadpool};
use crate::{connection, frame};
use std::io;
use std::net::TcpListener;

pub struct Server {
    thread_pool: threadpool::ThreadPool,
    listener: TcpListener,
}

impl Server {
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

    /// listen listens to incoming connections and process them.
    pub fn listen(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let conn = connection::Connection::new(stream);
                    match conn {
                        Ok(conn) => {
                            self.thread_pool.execute(move || handle_connection(conn));
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
        // 1. Read command
        let cmd_frame = conn.read_frame();
        match cmd_frame {
            Ok(cmd_frame) => {
                // 2. get command name
                let cmd_name = cmd::get_name(&cmd_frame);
                match cmd_name {
                    Ok(cmd_name) => {
                        // 3. create command type
                        let cmd_type = cmd::create_command(&cmd_name);
                        match cmd_type {
                            Some(cmd_type) => {
                                if let Some(cmd) = cmd_type {
                                } else {
                                }
                            }
                            None => {
                                eprintln!("unable to get command type");
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("failed to get command name: {}", e);
                        continue;
                    }
                }
            }
            Err(e) => match e {
                FrameError::EOF => {
                    println!("connection closed, client_ip: {}", conn.get_client_ip());
                    break;
                }
                _ => {
                    eprintln!("error processing frame: {}", e)
                }
            },
        }
    }
}
