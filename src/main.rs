use redisy::threadpool::ThreadPool;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main_threadpool() {
    let mut pool = ThreadPool::new(4).unwrap();
    for i in 0..10 {
        pool.execute(move || println!("Running job id {}", i));
    }
    pool.shutdown();
}

fn main() -> std::io::Result<()> {
    let mut pool = ThreadPool::new(2).unwrap();
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    println!("server listening on port 7878");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("connexion being handle by thread pool");
                pool.execute(move || handle_client(stream));
            }
            Err(e) => {
                eprintln!("connexion failed: {}", e);
            }
        }
    }
    pool.shutdown();
    Ok(())
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    loop {
        let bytes_read = stream
            .read(&mut buffer)
            .expect("Failed to read from socket");
        if bytes_read == 0 {
            return;
        }
        stream
            .write_all(&buffer[..bytes_read])
            .expect("Failed to write to socket");
        stream.flush().expect("Failed to flush");
        println!("{}", String::from_utf8(buffer.to_vec()).unwrap());
    }
}

