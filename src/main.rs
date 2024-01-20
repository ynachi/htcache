use htcache::{db, server};

fn main() -> std::io::Result<()> {
    let server = server::create_server(
        "127.0.0.1".to_string(),
        6379,
        120,
        16,
        8,
        db::EvictionPolicy::RANDOM,
    )?;
    server.listen();
    Ok(())
}
