use crate::db;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockWriteGuard};

pub struct LFU {
    // minimum frequency in the cache
    min_frequency: AtomicUsize,
    // frequency count long with values. Think of it as frequency bucket
    // usize is the frequency and VecDeque a bucket to objects with that frequency.
    frequency: RwLock<HashMap<usize, VecDeque<Arc<Mutex<db::Entry>>>>>,
}

impl LFU {
    pub fn new() -> Self {
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

    fn update_frequency_bucket(
        frequency_buckets: &mut RwLockWriteGuard<HashMap<usize, VecDeque<Arc<Mutex<db::Entry>>>>>,
        entry: &mut MutexGuard<db::Entry>,
    ) {
        if let Some(frequency_bucket) = frequency_buckets.get_mut(&entry.usage_freq) {
            frequency_bucket.retain(|k| k.lock().unwrap().key != entry.key);
        }
    }
}

impl Default for LFU {
    fn default() -> Self {
        LFU::new()
    }
}

impl db::Eviction for LFU {
    fn refresh(&mut self, entry: Arc<Mutex<db::Entry>>) {
        // remove from the bucket
        let mut frequency_buckets = self.frequency.write().unwrap();
        // create a clone before to be able to shadow entry variable.
        // We need
        // to clone because we might have to insert the clone in the frequency bucket.
        // Not cloning at this stage would requires
        // to give a different name to the locked entry.
        // We want to keep the same name because it makes the code clearer.
        let entry_clone = Arc::clone(&entry);
        let mut entry = entry.lock().unwrap();

        Self::update_frequency_bucket(&mut frequency_buckets, &mut entry);

        // increase entry usage and add it back to the bucket
        entry.usage_freq += 1;
        let new_frequency_bucket = frequency_buckets.entry(entry.usage_freq).or_default();
        new_frequency_bucket.push_front(entry_clone);

        // check if we need to update min_frequency
        if let Some(frequency_bucket) = frequency_buckets.get(&self.get_min_frequency()) {
            if frequency_bucket.is_empty() {
                self.incr_min_frequency();
            }
        }
    }

    // @TODO check if we need to update min_frequency after evict, not sure yet
    fn evict(&mut self) -> Option<Arc<Mutex<db::Entry>>> {
        // find the next element to evict first
        // remove it from the corresponding bucket
        // return a reference to it
        let mut frequency_buckets = self.frequency.write().unwrap();
        if let Some(min_frequency_bucket) = frequency_buckets.get_mut(&self.get_min_frequency()) {
            return match min_frequency_bucket.pop_back() {
                Some(entry) => {
                    min_frequency_bucket
                        .retain(|k| k.lock().unwrap().key != entry.lock().unwrap().key);
                    Some(Arc::clone(&entry))
                }
                None => None,
            };
        }
        None
    }

    /// remove removes an entry from the eviction data structure.
    /// It is typically called after remove on the storage backend.
    /// When we call remove on the storage backend, it returned the removed item,
    /// which can then be used in the call of this method.
    /// The reason we do that way is that both structures should be tied for proper eviction.
    fn remove(&mut self, entry: Arc<Mutex<db::Entry>>) {
        let mut frequency_buckets = self.frequency.write().unwrap();
        let mut entry = entry.lock().unwrap();
        Self::update_frequency_bucket(&mut frequency_buckets, &mut entry);
    }
}
