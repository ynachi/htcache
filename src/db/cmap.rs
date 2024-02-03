use crate::db;
use rustc_hash::FxHashMap;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct Bucket {
    storage: FxHashMap<String, String>,
    _eviction_state: BinaryHeap<(Instant, String)>,
}

impl Bucket {
    fn new(capacity: usize) -> Bucket {
        Bucket {
            storage: FxHashMap::default(),
            _eviction_state: BinaryHeap::with_capacity(capacity),
        }
    }

    fn get_value_by_key(&self, key: &str) -> Option<&String> {
        self.storage.get(key)
    }

    fn add_entry_or_update(&mut self, key: String, value: String) -> Option<String> {
        // self._eviction_state.push((Instant::now(), key.clone()));
        self.storage.insert(key, value)
    }

    fn remove_entry(&mut self, key: &str) -> usize {
        if self.storage.remove(key).is_some() {
            return 1;
        }
        0
    }

    fn contains_key(&self, key: &str) -> bool {
        self.storage.contains_key(key)
    }
}

pub struct CMap {
    shards: Vec<Arc<Mutex<Bucket>>>,
    // shard size should be a power of two
    shard_count: usize,
    size: AtomicUsize,
}

impl Debug for CMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cmap{{ shard_count: {} }}", self.shard_count)
    }
}

impl CMap {
    pub fn shard_count(&self) -> usize {
        self.shard_count
    }

    pub fn get_shard_by_key(&self, key: &str) -> Arc<Mutex<Bucket>> {
        let index = self.get_shard_index(key);
        Arc::clone(&self.shards[index])
    }

    pub fn get_shard_by_index(&self, index: usize) -> Option<Arc<Mutex<Bucket>>> {
        if index >= self.shard_count {
            None
        } else {
            Some(Arc::clone(&self.shards[index]))
        }
    }

    fn get_shard_index(&self, key: &str) -> usize {
        let key_hash = db::calculate_hash(&key);
        let shard_bits = self.shard_count.trailing_zeros();
        let shard_mask = (1 << shard_bits) - 1;
        (key_hash & shard_mask) as usize
    }

    pub fn new(shard_count: usize, bucket_size: usize) -> io::Result<Self> {
        if !shard_count.is_power_of_two() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "shard_count must be a power of 2",
            ));
        }
        let mut shards = Vec::with_capacity(shard_count);
        for _ in 0..shard_count {
            let shard = Arc::new(Mutex::new(Bucket::new(bucket_size)));
            shards.push(shard);
        }
        Ok(Self {
            shards,
            shard_count,
            size: Default::default(),
        })
    }

    pub fn set_kv(&self, key: &str, value: &str) {
        let previous_value = self
            .get_shard_by_key(key)
            .lock()
            .unwrap()
            .add_entry_or_update(key.to_string(), value.to_string());

        if previous_value.is_none() {
            self.size.fetch_add(1, Ordering::SeqCst);
        };
    }

    pub fn get_value(&self, key: &str) -> Option<String> {
        self.get_shard_by_key(key)
            .lock()
            .unwrap()
            .get_value_by_key(key)
            .cloned()
    }

    /// del_entries remove entries and return a vector of shards where the deletion happened with the count of items deleted.
    /// This method could be more simple, but we want to group keys to avoid locking/de-locking the same shared many times.
    pub fn del_entries(&self, keys: &Vec<String>) -> HashMap<usize, usize> {
        let mut ans = HashMap::new();
        let shard_key_mapping = self.get_shard_key_mapping(keys);

        for (shard_id, keys) in shard_key_mapping {
            let count = self.delete_shard_entries(shard_id, keys);
            if count > 0 {
                ans.insert(shard_id, count);
            }
        }

        ans
    }

    /// get_shard_key_mapping to group the keys, according to the shard, they belong to
    fn get_shard_key_mapping(&self, keys: &Vec<String>) -> HashMap<usize, HashSet<String>> {
        let mut shard_key_mapping: HashMap<usize, HashSet<String>> = HashMap::new();
        for key in keys {
            let shard_index = self.get_shard_index(key);
            shard_key_mapping
                .entry(shard_index)
                .or_default()
                .insert(key.clone());
        }
        shard_key_mapping
    }

    /// delete_shard_entries to delete entries from a shard and return the count of deleted items
    fn delete_shard_entries(&self, shard_id: usize, keys: HashSet<String>) -> usize {
        let mut count = 0;

        if let Some(shard) = self.get_shard_by_index(shard_id) {
            let mut shard = shard.lock().unwrap();
            for key in keys {
                count += shard.remove_entry(&key)
            }
        }
        self.size.fetch_sub(count, Ordering::SeqCst);

        count
    }

    fn contains_key(&self, key: &str) -> bool {
        self.get_shard_by_key(key).lock().unwrap().contains_key(key)
    }

    pub fn size(&self) -> usize {
        self.size.load(Ordering::SeqCst)
    }

    /// apply_fn_mut_shards applies a mutable function to all shards of the Cmap.
    pub fn apply_mut_fn_shards<F: Fn(&mut Bucket) -> T, T>(&self, func: F) -> Vec<T> {
        self.shards
            .iter()
            .map(|shard| {
                let mut data = shard.lock().unwrap();
                func(&mut data)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmap() {
        // Create a CMap
        let cmap = CMap::new(16, 100).unwrap();

        // Check shard count
        assert_eq!(cmap.shard_count(), 16);

        // Set key-value pairs
        cmap.set_kv("key1", "value1");
        cmap.set_kv("key2", "value2");

        // Test size
        assert_eq!(cmap.size(), 2);

        // Test `get_value`
        assert_eq!(cmap.get_value("key1"), Some("value1".to_string()));
        assert_eq!(cmap.get_value("key2"), Some("value2".to_string()));

        // Check that keys are distributed amongst the shards
        let mut keys_in_shards = 0;
        for i in 0..cmap.shard_count() {
            let shard = cmap.get_shard_by_index(i).unwrap();
            let locked_shard = shard.lock().unwrap();
            if locked_shard.contains_key("key1") || locked_shard.contains_key("key2") {
                keys_in_shards += 1;
            }
        }

        assert!(keys_in_shards > 0);

        // Test `del_entries`
        let keys = vec!["key1".to_string(), "key2".to_string()];
        let deleted = cmap.del_entries(&keys);
        for (_, count) in deleted {
            assert!(count > 0);
        }

        assert_eq!(cmap.size(), 0);
    }
}
