mod cache;
mod cmap;

pub use cache::create_cache;
pub use cache::Cache;
pub use cache::State;

extern crate rand;
use std::time::Instant;

pub struct CacheEntry {
    key: String,
    value: String,
    // at insertion, `Instant::now() + ttl` upon creation
    expiration_time: Instant,
}

impl CacheEntry {
    fn new(key: &str, value: &str, expiration_time: Instant) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
            expiration_time,
        }
    }
    // there is no update in place, we just replace the entire entry in case of update

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expiration_time
    }
}
