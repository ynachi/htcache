use htcache::server;

/// main is a placeholder for testing the application for now
pub fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::try_init().expect("unable to initialize logging");
    let server = server::create_server("127.0.0.1".to_string(), 6379, 100, 10000000, 32, 80)?;
    server.listen();
    Ok(())
}
