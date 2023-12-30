pub mod lfu;
mod lru;
mod map;

use crate::db;
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::Instant;

/// Storage defines operations needed on storage to be used as a cache backend.
trait Storage {
    /// is_full checks if the storage is full and needs eviction.
    fn is_full(&self) -> bool;

    /// get return the entry with the given key if found.
    /// It does not handle eviction logic.
    /// It is suppose
    /// to just call the get method of the underline data structure.
    /// It can return an error if there is some mutex lock issue.
    /// Thus, it might be a good idea for the calling function to implement some retry logic.
    fn get(&self, key: &str) -> Option<Arc<Entry>>;

    /// set set an entry. This method should always be successfully. Returns a reference to the set key.
    fn set(&mut self, key: &str, value: &str) -> Arc<Entry>;

    /// delete remove an entry given it key. Returns a reference to the deleted key.
    fn remove(&mut self, key: &str) -> Option<Arc<Entry>>;
}

/// Eviction defines common operations for an eviction policy.
trait Eviction {
    /// refresh updates the position of this key in the caching system.
    /// For example, in the case of LRU,
    /// it makes sure that the most the less used key is evicted first.
    fn refresh(&mut self, entry: Arc<Entry>);

    /// evict removes the element to evict to the Eviction struct and a reference to the evicted element.
    /// This reference is then used to remove the entry in the storage backend.
    fn evict(&mut self) -> Option<Arc<Entry>>;

    /// remove removes an entry in the eviction structure. Not necessary an eviction.
    fn remove(&mut self, entry: Arc<db::Entry>);
}

/// Cache combines storage and eviction to create a cache instance.
struct Cache<S: Storage, E: Eviction> {
    storage: S,
    eviction: E,
}

impl<S: Storage, E: Eviction> Cache<S, E> {
    // call self.storage.get();
    fn get(&mut self, key: String) -> Option<String> {
        // If the key exist, return it and refresh it position in the eviction policy.
        if let Some(entry) = self.storage.get(&key) {
            self.eviction.refresh(entry);
            return Some(entry.value.to_string());
        }
        None
    }

    // call self.storage.set(), evict if needed
    fn set(&mut self, key: String, value: String) {
        // check if the entry is in the cache, update and refresh
        if let Some(entry) = self.storage.get(&key) {
            entry.value = value;
            self.eviction.refresh(entry);
            return;
        }
        // if full, evict
        if self.storage.is_full() {
            if let Some(entry) = self.eviction.evict() {
                self.storage.remove(&entry.key);
            }
        }
        // now set
        let entry = self.storage.set(&key, &value);
        self.eviction.refresh(entry);
    }

    // call self.storage.remove(); also remove in Eviction
    fn remove(&mut self, key: String) {
        let entry = self.storage.remove(&key);
        if let Some(entry) = entry {
            self.eviction.remove(entry)
        }
    }
}

/// CacheBuilder is a struct used to build a cache.
/// # Example
///
/// ```
/// let storage_instance = HashMapCache::new();
///
/// let eviction_instance = LRUCache::new();
///
/// // Create the cache with chosen mechanisms
/// let cache = CacheBuilder::new()
///     .with_storage(Box::new(storage_instance))
///     .with_eviction(Box::new(eviction_instance))
///     .build();
/// ```
pub struct CacheBuilder<S: Storage, E: Eviction> {
    storage: Option<S>,
    eviction: Option<E>,
}

impl<S: Storage, E: Eviction> CacheBuilder<S, E> {
    pub fn new() -> Self {
        CacheBuilder {
            storage: None,
            eviction: None,
        }
    }

    pub fn with_storage(mut self, storage: S) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn with_eviction(mut self, eviction: E) -> Self {
        self.eviction = Some(eviction);
        self
    }

    pub fn build(self) -> Cache<S, E> {
        Cache {
            storage: self.storage.expect("No Storage"),
            eviction: self.eviction.expect("No Eviction"),
        }
    }
}

/// Entry represents a cache item
#[derive(Debug, Clone)]
pub struct Entry {
    key: String,
    value: String,
    usage_freq: usize,
    expires_at: Option<Instant>,
}

impl Entry {
    pub fn new(key: String, value: String) -> Self {
        Self {
            key,
            value,
            usage_freq: 0,
            expires_at: None,
        }
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.usage_freq == other.usage_freq
    }
}

impl Eq for Entry {}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.usage_freq.cmp(&other.usage_freq)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
