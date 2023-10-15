use crate::threadpool;
use std::net::TcpListener;
use std::{io, net};

pub struct Server {
    ip_address: net::IpAddr,
    port: u16,
    thread_pool: threadpool::ThreadPool,
}

impl Server {
    pub fn start(&self) -> io::Result<TcpListener> {
        TcpListener::bind((self.ip_address.to_string(), self.port))
    }

    pub fn serve(&self, listener: TcpListener) {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {}
                Err(e) => {}
            }
            //1. interpret stream to extract instruction (GET or PUT command)
            //2. execute corresponding command
            // 3. send the response back. So maybe ThreadPool::execute needs to be updated to
            // return a result to the caller
        }
    }
}
