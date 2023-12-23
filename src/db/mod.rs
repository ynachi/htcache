mod lfu;

use crate::error;
use std::cmp::Ordering;
use std::time::Instant;

pub trait Database {
    /// get return the entry with the given key if found
    fn get(&mut self, key: &str) -> Result<Option<Entry>, error::DatabaseError>;

    /// set set an entry. This method should always be successfully.
    fn set(&mut self, key: String, data: Entry) -> Result<(), error::DatabaseError>;

    /// delete remove an entry given it key
    fn delete(&mut self, key: String) -> Result<(), error::DatabaseError>;

    /// iterate returns an iterator to entries in the database.
    /// These are useful for routines which need to process all the data of the db like cleanups.
    fn iterate(&self) -> Box<dyn Iterator<Item = Entry>>;

    /// chose_evict returns the key of the items to be evicted next as per the caching algorithm.
    fn chose_evict(&self) -> Result<Option<String>, error::DatabaseError>;
}

/// Entry represents a cache item
#[derive(Debug, Clone)]
pub struct Entry {
    key: String,
    data: String,
    usage_freq: usize,
    expires_at: Option<Instant>,
}

impl Entry {
    pub fn update_frequency(&mut self) {
        self.usage_freq += 1
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
