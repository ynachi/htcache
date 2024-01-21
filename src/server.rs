use crate::error::{CommandError, FrameError};
use crate::threadpool;
use crate::{connection, db};
use std::fmt::{Debug, Formatter};
use std::io;
use std::net::TcpListener;
use std::sync::Arc;
use tracing::info;

#[derive(Debug)]
pub struct Server {
    thread_pool: threadpool::ThreadPool,
    tcp_listener: TcpListener,
    htcache: Arc<db::HTCache>,
    // @ TODO: uncomment and implement
    // max_connection: AtomicUsize,
    // is_shutdown: AtomicBool,
}

/// `create_server` return a Result instead of the actual type.
/// It is required in this case because creating a new server requires
/// to prepare threads that it will use to process the requests.
/// And, creating threads are likely to fail for reasons related to the OS.
pub fn create_server(
    server_ip: String,
    server_port: u16,
    worker_count: usize,
    page_space: u32,
    entry_space: u32,
    eviction_policy: db::EvictionPolicy,
) -> io::Result<Server> {
    let thread_pool = threadpool::ThreadPool::new(worker_count)?;
    let tcp_listener = TcpListener::bind((server_ip, server_port))?;

    info!("htcache server initialized");

    let htcache = Arc::new(db::HTCache::new(page_space, entry_space, eviction_policy));
    Ok(Server {
        thread_pool,
        tcp_listener,
        htcache,
    })
}

impl Server {
    /// listen listens to incoming connections and process them.
    pub fn listen(&self) {
        info!("{:?}", self);
        info!("htcache server ready for new connections");
        for stream in self.tcp_listener.incoming() {
            let stream = match stream {
                Ok(stream) => stream,
                Err(e) => {
                    self.log_error("failed to establish connection", e);
                    continue;
                }
            };

            let htcache = Arc::clone(&self.htcache);
            let mut conn = match connection::Connection::new(stream, htcache) {
                Ok(conn) => conn,
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
        eprintln!("{}: {}", message, error);
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
