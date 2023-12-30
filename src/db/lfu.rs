use crate::db;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

struct LFU {
    // minimum frequency in the cache
    min_frequency: AtomicUsize,
    // frequency count long with values. Think of it as frequency bucket
    // usize is the frequency and VecDeque a bucket to objects with that frequency.
    frequency: RwLock<HashMap<usize, VecDeque<Arc<db::Entry>>>>,
}

impl LFU {
    fn new() -> Self {
        LFU {
            min_frequency: AtomicUsize::new(0),
            frequency: RwLock::new(HashMap::new()),
        }
    }

    fn get_min_frequency(&self) -> usize {
        self.min_frequency.load(Ordering::SeqCst)
    }

    fn incr_min_frequency(&self) -> usize {
        self.min_frequency.fetch_add(1, Ordering::SeqCst)
    }
}
impl db::Eviction for LFU {
    fn refresh(&mut self, entry: Arc<db::Entry>) {
        // remove from the bucket
        let mut guard = self.frequency.write().unwrap();
        if let Some(mut frequency_bucket) = guard.get(&entry.usage_freq) {
            frequency_bucket.retain(|&k| k != entry);
        }

        // increase entry usage and add it back to the bucket
        entry.usage_freq += 1;
        let new_frequency_bucket = guard.entry(entry.usage_freq).or_insert(VecDeque::new());
        new_frequency_bucket.push_front(entry);

        // check if we need to update min_frequency
        if let Some(frequency_bucket) = guard.get(&self.get_min_frequency()) {
            if frequency_bucket.is_empty() {
                self.incr_min_frequency();
            }
        }
    }

    // @TODO check if we need to update min_frequency after evict, not sure yet
    fn evict(&mut self) -> Option<Arc<db::Entry>> {
        // find the next element to evict first
        // remove it from the corresponding bucket
        // return a reference to it
        let guard = self.frequency.write().unwrap();
        if let Some(mut min_frequency_bucket) = guard.get(&self.get_min_frequency()) {
            let entry = min_frequency_bucket.pop_back();
            if entry.is_some() {
                min_frequency_bucket.retain(|&k| k != entry.unwrap());
            }
            return entry;
        }
        None
    }

    /// remove removes an entry from the eviction data structure.
    /// It is typically called after remove on the storage backend.
    /// When we call remove on the storage backend, it returned the removed item,
    /// which can then be used in the call of this method.
    /// The reason we do that way is that both structures should be tied for proper eviction.
    fn remove(&mut self, entry: Arc<db::Entry>) {
        let guard = self.frequency.write().unwrap();
        if let Some(mut frequency_bucket) = guard.get(&entry.usage_freq) {
            frequency_bucket.retain(|&k| k != entry);
        }
    }
}
