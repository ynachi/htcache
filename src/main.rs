use redisy::server;

fn main() -> std::io::Result<()> {
    let server = server::new("127.0.0.1".to_string(), 6379, 8, 5000, "MAP", "LRU")?;
    server.listen();
    Ok(())
}
