use crate::error::{FrameError, HandleCommandError};
use crate::threadpool;
use crate::{connection, db};
use std::fmt::Debug;
use std::io;
use std::net::TcpListener;
use std::sync::Arc;
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct Server {
    thread_pool: threadpool::ThreadPool,
    tcp_listener: TcpListener,
    cache: db::Cache,
    // @ TODO: uncomment and implement
    // max_connection: AtomicUsize,
    // is_shutdown: AtomicBool,
}

/// `create_server` return a Result instead of the actual type.
/// It is required in this case because creating a new server requires
///  preparing threads that it will use to process the requests.
/// And, creating threads are likely to fail for reasons related to the OS.
pub fn create_server(
    server_ip: String,
    server_port: u16,
    worker_count: usize,
    cache_capacity: usize,
    shard_count: usize,
    eviction_threshold: u8,
) -> io::Result<Server> {
    let thread_pool = threadpool::ThreadPool::new(worker_count)?;
    let tcp_listener = TcpListener::bind((server_ip, server_port))?;

    info!("htcache server initialized");

    let cache = db::create_cache(cache_capacity, shard_count, eviction_threshold)?;

    Ok(Server {
        thread_pool,
        tcp_listener,
        cache,
    })
}

impl Server {
    /// listen listens to incoming connections and process them.
    pub fn listen(&self) {
        // show server's info to the user
        info!("{:?}", self);
        info!("htcache server ready for new connections");

        for stream in self.tcp_listener.incoming() {
            let stream = match stream {
                Ok(stream) => stream,
                Err(e) => {
                    error!(
                        error_message = e.to_string(),
                        "failed to establish connection"
                    );
                    self.log_error("failed to establish connection", e);
                    continue;
                }
            };

            let data_store = Arc::clone(&self.cache.db());
            let mut conn = match connection::Connection::new(stream, data_store) {
                Ok(conn) => {
                    debug!(
                        remote_address = conn.get_client_ip(),
                        "new connection created"
                    );
                    conn
                }
                Err(e) => {
                    self.log_error("failed to create connection object", e);
                    continue;
                }
            };

            self.thread_pool
                .execute(move || handle_connection(&mut conn));
        }
    }

    fn log_error(&self, message: &str, error: impl std::fmt::Display) {
        error!(error_message = error.to_string(), message);
    }
}

fn handle_connection(conn: &mut connection::Connection) {
    loop {
        match conn.handle_command() {
            Ok(_) => {}
            Err(HandleCommandError::Frame(FrameError::EOF)) => {
                debug!(
                    remote_address = conn.get_client_ip(),
                    "client gracefully closed connection"
                );
                break;
            }
            Err(e) => {
                debug!(
                    // Internal error, log but don't send to client.
                    error_message = e.to_string(),
                    "error processing command  frame or name"
                );
            }
        };
    }
}
