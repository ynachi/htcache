mod htcache;
mod htpage;

pub use htcache::*;
pub use htpage::HTPage;

extern crate rand;
use rand::Rng;
use std::time::Instant;

pub enum EvictionPolicy {
    CLOCK,
    LFU,
    LRU,
    RANDOM,
}

#[derive(Debug)]
pub struct HTPageEntry {
    key: String,
    value: String,
    // frequency count
    freq_count: usize,
    expires_at: Option<Instant>,
}

impl HTPageEntry {
    pub fn new(key: &str, value: &str, expires_at: Option<Instant>) -> Self {
        HTPageEntry {
            key: key.to_string(),
            value: value.to_string(),
            freq_count: 0,
            expires_at,
        }
    }

    fn increment_frequency(&mut self) {
        self.freq_count += 1
    }

    fn update_fields(&mut self, value: &str) {
        self.value = value.to_string();
        self.increment_frequency();
    }
}

pub type EvictionFn = fn(&mut [Option<HTPageEntry>], key: &str, value: &str);

/// get_choose_evict_fn returns a evict replace and evict function. Our eviction strategy is to replace a kv by an
/// incoming one, when we could not find a suitable place for it.
pub fn get_choose_evict_fn(eviction_policy: EvictionPolicy) -> EvictionFn {
    match eviction_policy {
        EvictionPolicy::CLOCK => clock_choose_evict,
        EvictionPolicy::LFU => lfu_choose_evict,
        EvictionPolicy::LRU => lru_choose_evict,
        EvictionPolicy::RANDOM => random_evict_and_replace,
    }
}

fn clock_choose_evict(pages: &mut [Option<HTPageEntry>], key: &str, value: &str) {
    unimplemented!()
    // Implement it...
}

fn lfu_choose_evict(pages: &mut [Option<HTPageEntry>], key: &str, value: &str) {
    unimplemented!()
}

fn lru_choose_evict(pages: &mut [Option<HTPageEntry>], key: &str, value: &str) {
    unimplemented!()
}
fn random_evict_and_replace(page: &mut [Option<HTPageEntry>], key: &str, value: &str) {
    if page.is_empty() {
        dbg!("page is empty and this should normally not happen");
        return;
    }

    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..page.len());

    let new_entry = HTPageEntry::new(key, value, None);

    if let Some(entry) = page.get_mut(index) {
        *entry = Some(new_entry);
    }
}
