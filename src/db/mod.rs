mod cache;
pub mod cmap;
use rustc_hash::FxHasher;

pub use cache::create_cache;
pub use cache::Cache;
pub use cache::State;
use std::hash::{Hash, Hasher};

extern crate rand;
use std::time::Instant;

#[derive(Clone)]
pub struct CacheEntry {
    key: String,
    value: String,
    // at insertion, `Instant::now() + ttl` upon creation
    expiration_time: Instant,
}

impl CacheEntry {
    pub fn new(key: &str, value: &str, expiration_time: Instant) -> Self {
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

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut hasher = FxHasher::default();
    t.hash(&mut hasher);
    hasher.finish()
}
