use crate::error::{CommandError, FrameError};
use crate::threadpool;
use crate::{connection, db};
use std::io;
use std::net::TcpListener;

pub struct Server<S: db::Storage, E: db::Eviction> {
    thread_pool: threadpool::ThreadPool,
    listener: TcpListener,
    cache: db::Cache<S, E>,
}

/// create_storage_type create storage type from name.
/// We fallback to a default storage if the given nme is not valid.
pub fn create_storage_type(name: &str) -> db::storage::StorageType {
    match name {
        "MAP" => db::storage::StorageType::Map,
        // default to map
        _ => db::storage::StorageType::Map,
    }
}

/// create_eviction_type creates eviction type from name.
/// We fallback to a default eviction if the given nme is not valid.
pub fn create_eviction_type(name: &str) -> db::eviction::EvictionPolicyType {
    match name {
        "LFU" => db::eviction::EvictionPolicyType::LFU,
        // Default to LFU
        _ => db::eviction::EvictionPolicyType::LFU,
    }
}

/// `new` return a Result instead of the actual type.
/// It is required in this case because creating a new server requires
/// to prepare threads that it will use to process the requests.
/// And, creating threads are likely to fail for reasons related to the OS.
pub fn new(
    ip: String,
    port: u16,
    num_workers: usize,
    max_item_size: usize,
    storage_type_name: &str,
    eviction_policy_name: &str,
) -> io::Result<Server<impl db::Storage, impl db::Eviction>> {
    let thread_pool = threadpool::ThreadPool::new(num_workers)?;
    let listener = TcpListener::bind((ip, port))?;

    let storage_type = create_storage_type(storage_type_name);
    let eviction_type = create_eviction_type(eviction_policy_name);
    let storage = db::storage::create_storage(storage_type, max_item_size);
    let eviction = db::eviction::create_eviction_policy(eviction_type);

    let cache = db::CacheBuilder::new()
        .with_storage(storage)
        .with_eviction(eviction)
        .build();

    Ok(Server {
        thread_pool,
        listener,
        cache,
    })
}

impl<S: db::Storage, E: db::Eviction> Server<S, E> {
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
