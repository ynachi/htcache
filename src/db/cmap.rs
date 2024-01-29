use crate::db::CacheEntry;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::{Arc, RwLock};

pub struct CMap {
    shards: Vec<Arc<RwLock<HashMap<String, CacheEntry>>>>,
    // shard size should be a power of two
    shard_count: usize,
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

    fn calculate_hash<T: Hash>(&self, t: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get_shard_by_key(&self, key: &str) -> Arc<RwLock<HashMap<String, CacheEntry>>> {
        let index = self.get_shard_index(key);
        Arc::clone(&self.shards[index])
    }

    pub fn get_shard_by_index(
        &self,
        index: usize,
    ) -> Option<Arc<RwLock<HashMap<String, CacheEntry>>>> {
        if index >= self.shard_count {
            None
        } else {
            Some(Arc::clone(&self.shards[index]))
        }
    }

    fn get_shard_index(&self, key: &str) -> usize {
        let key_hash = self.calculate_hash(&key);
        let shard_bits = self.shard_count.trailing_zeros();
        let shard_mask = (1 << shard_bits) - 1;
        (key_hash & shard_mask) as usize
    }

    pub fn create(shard_count: usize) -> io::Result<Self> {
        if !shard_count.is_power_of_two() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "shard_count must be a power of 2",
            ));
        }
        let mut shards = Vec::with_capacity(shard_count);
        for _ in 0..shard_count {
            let shard = Arc::new(RwLock::new(HashMap::new()));
            shards.push(shard);
        }
        Ok(Self {
            shards,
            shard_count,
        })
    }

    pub fn set_kv(&self, key: &str, value: CacheEntry) {
        let shard = self.get_shard_by_key(key);
        shard.write().unwrap().insert(key.to_string(), value);
    }

    pub fn get_value(&self, key: &str) -> Option<String> {
        let shard = self.get_shard_by_key(key);
        if let Some(entry) = shard.read().unwrap().get(key) {
            return Some(entry.value.clone());
        }
        None
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
                .or_insert_with(HashSet::new)
                .insert(key.clone());
        }
        shard_key_mapping
    }

    /// delete_shard_entries to delete entries from a shard and return the count of deleted items
    fn delete_shard_entries(&self, shard_id: usize, keys: HashSet<String>) -> usize {
        let mut count = 0;

        if let Some(shard) = self.get_shard_by_index(shard_id) {
            let mut shard = shard.write().unwrap();
            for key in keys {
                if shard.remove(&key).is_some() {
                    count += 1
                }
            }
        }

        count
    }

    fn contains_key(&self, key: &str) -> bool {
        let shard = self.get_shard_by_key(key);
        let read_guard = shard.read().unwrap();
        read_guard.contains_key(key)
    }

    fn size_per_shard(&self) -> HashMap<usize, usize> {
        let mut ans = HashMap::new();
        for (i, shard) in self.shards.iter().enumerate() {
            let size = shard.read().unwrap().len();
            ans.insert(i, size);
        }
        ans
    }

    pub fn size(&self) -> usize {
        let shard_sizes = self.size_per_shard();
        shard_sizes.values().sum()
    }

    /// apply_fn_mut_shards applies a mutable function to all shards of the Cmap.
    pub fn apply_mut_fn_shards<F, T>(&self, func: F) -> Vec<T>
    where
        F: Fn(&mut HashMap<String, CacheEntry>) -> T,
    {
        self.shards
            .iter()
            .map(|shard| {
                let mut data = shard.write().unwrap();
                func(&mut data)
            })
            .collect()
    }
}
