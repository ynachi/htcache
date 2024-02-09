use htcache::server;
use metrics::describe_counter;

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::try_init().expect("unable to initialize logging");
    let server = server::create_server("127.0.0.1".to_string(), 6379, 5000, 8, 80).await?;
    server.listen().await;
    Ok(())
}
