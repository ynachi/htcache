use htcache::server;

pub fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::try_init().expect("unable to initialize logging");
    let server = server::create_server("127.0.0.1".to_string(), 6379, 10000000, 16, 80)?;
    server.listen();
    Ok(())
}
