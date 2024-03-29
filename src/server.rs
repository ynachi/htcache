use crate::connection::Connection;
use crate::db::State;
use crate::error::{FrameError, HandleCommandError};
use crate::{db, threadpool};
use std::fmt::Debug;
use std::io;
use std::net::{TcpListener, TcpStream};
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
/// And, creating threads is likely to fail for reasons related to the OS.
pub fn create_server(
    server_ip: String,
    server_port: u16,
    worker_count: usize,
    cache_capacity: usize,
    shard_count: usize,
    eviction_threshold: u8,
) -> io::Result<Server> {
    let ip = format!("{}:{}", server_ip, server_port);
    let tcp_listener = TcpListener::bind(ip)?;
    let thread_pool = crate::threadpool::ThreadPool::new(worker_count)?;

    info!("htcache server initialized");
    let cache = db::create_cache(cache_capacity, shard_count, eviction_threshold)?;

    Ok(Server {
        thread_pool,
        tcp_listener,
        cache,
    })
}

impl Server {
    /// listen listens to incoming connections and process them. Each connection is processed in
    /// a separate thread.
    /// We started with our own implementation of a thread pool.
    /// We then, moved to tokio green threads.
    pub fn listen(&self) {
        // show server's info to the user
        info!("{:?}", self);
        info!("htcache server ready for new connections");
        loop {
            let conn_string = self.tcp_listener.accept();
            match conn_string {
                Ok((socket, addr)) => {
                    debug!("new connection established: {}", addr);
                    // Process each socket in parallel.
                    // Each connection needs to read and update the state so create a shared reference of the state
                    // and share it to the process_socket function.
                    let db = self.cache.db();
                    self.thread_pool.execute(move || {
                        process_socket(socket, db);
                    });
                }
                Err(e) => {
                    log_error("unable to establish new connection", e);
                }
            }
        }
    }
}

fn process_socket(socket: TcpStream, db: Arc<State>) {
    let conn = Connection::new(socket, db);
    match conn {
        Ok(mut conn) => {
            process_commands(&mut conn);
        }
        Err(e) => {
            log_error("failed to create connection object", e);
        }
    }
}

fn process_commands(conn: &mut Connection) {
    loop {
        match conn.handle_command() {
            Ok(_) => {}
            Err(HandleCommandError::Frame(FrameError::EOF)) => {
                debug!(
                    remote_address = "conn.get_client_ip()",
                    "client gracefully closed connection"
                );
                break;
            }
            Err(e) => {
                debug!(
                    // Internal error, log but don't send to a client.
                    error_message = e.to_string(),
                    "error processing command  frame or name"
                );
            }
        };
    }
}

fn log_error(message: &str, error: impl std::fmt::Display) {
    error!(error_message = error.to_string(), message);
}
