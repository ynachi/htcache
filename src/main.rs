use redisy::server;

fn main() -> std::io::Result<()> {
    let server = server::create_server("127.0.0.1".to_string(), 6379, 80, 5000, "MAP", "LRU")?;
    server.listen();
    Ok(())
}
