// use crate::error::{CommandError, FrameError};
// use crate::threadpool;
// use crate::{connection, db};
// use std::io;
// use std::net::TcpListener;
//
// pub struct Server<E: db::Eviction> {
//     thread_pool: threadpool::ThreadPool,
//     listener: TcpListener,
//     cache: db::Cache<E>,
//     // @ TODO: uncomment and implement
//     // max_connection: AtomicUsize,
//     // is_shutdown: AtomicBool,
// }
//
// /// create_storage_type create storage type from name.
// /// We fallback to a default storage if the given nme is not valid.
// pub fn create_storage_type(name: &str) -> db::storage::StorageType {
//     match name {
//         "MAP" => db::storage::StorageType::Map,
//         // default to map
//         _ => db::storage::StorageType::Map,
//     }
// }
//
// /// create_eviction_type creates eviction type from name.
// /// We fallback to a default eviction if the given nme is not valid.
// pub fn create_eviction_type(name: &str) -> db::eviction::EvictionPolicyType {
//     match name {
//         "LFU" => db::eviction::EvictionPolicyType::LFU,
//         // Default to LFU
//         _ => db::eviction::EvictionPolicyType::LFU,
//     }
// }
//
// /// `create_server` return a Result instead of the actual type.
// /// It is required in this case because creating a new server requires
// /// to prepare threads that it will use to process the requests.
// /// And, creating threads are likely to fail for reasons related to the OS.
// pub fn create_server(
//     server_ip: String,
//     server_port: u16,
//     worker_count: usize,
//     max_size: usize,
//     storage_name: &str,
//     eviction_policy: &str,
// ) -> io::Result<Server<impl db::Eviction>> {
//     let pool = threadpool::ThreadPool::new(worker_count)?;
//     let tcp_listener = TcpListener::bind((server_ip, server_port))?;
//     let storage = create_storage_type(storage_name);
//     let eviction = create_eviction_type(eviction_policy);
//     let server_storage = db::storage::create_storage(storage, max_size);
//     let server_eviction = db::eviction::create_eviction_policy(eviction);
//     let cache = db::create_cache(server_storage, server_eviction);
//     build_server(pool, tcp_listener, cache)
// }
//
// fn build_server<E: db::Eviction>(
//     thread_pool: threadpool::ThreadPool,
//     listener: TcpListener,
//     cache: db::Cache<E>,
// ) -> io::Result<Server<impl db::Eviction>> {
//     Ok(Server {
//         thread_pool,
//         listener,
//         cache,
//     })
// }
//
// impl<E: db::Eviction> Server<E> {
//     /// listen listens to incoming connections and process them.
//     pub fn listen(&self) {
//         for stream in self.listener.incoming() {
//             let stream = match stream {
//                 Ok(stream) => stream,
//                 Err(e) => {
//                     self.log_error("failed to establish connection", e);
//                     continue;
//                 }
//             };
//
//             let mut conn = match connection::Connection::new(stream) {
//                 Ok(conn) => conn,
//                 Err(e) => {
//                     self.log_error("failed to create connection object", e);
//                     continue;
//                 }
//             };
//
//             self.thread_pool
//                 .execute(move || handle_connection(&mut conn));
//         }
//     }
//
//     fn log_error(&self, message: &str, error: impl std::fmt::Display) {
//         eprintln!("{}: {}", message, error);
//     }
// }
//
// fn handle_connection(conn: &mut connection::Connection) {
//     loop {
//         match conn.handle_command() {
//             Ok(_) => {}
//             Err(FrameError::EOF) => break,
//             Err(e) => {
//                 conn.send_error(&CommandError::FrameDecode(e));
//             }
//         };
//     }
// }
