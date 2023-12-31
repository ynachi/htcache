use crate::db;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// Map is a std::collection::HashMap based storage for the cache service.
pub struct Map {
    max_items: usize,
    size: AtomicUsize,
    // arc because we need to share the reference of the same entry with the eviction struct.
    data: RwLock<HashMap<String, Arc<Mutex<db::Entry>>>>,
}

impl Map {
    pub fn new(max_items: usize) -> Self {
        Self {
            max_items,
            size: AtomicUsize::new(0),
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl db::Storage for Map {
    fn is_full(&self) -> bool {
        self.size.load(Ordering::SeqCst) >= self.max_items
    }

    fn get(&self, key: &str) -> Option<Arc<Mutex<db::Entry>>> {
        // Calling unwrap here is Ok because we want to panic is the mutex gets poisoned.
        // It is the right thing to do.
        let data_map = self.data.read().unwrap();
        if let Some(entry) = data_map.get(key) {
            return Some(Arc::clone(entry));
        }
        None
    }

    fn set(&mut self, key: &str, value: &str) -> Arc<Mutex<db::Entry>> {
        let mut data_map = self.data.write().unwrap();
        let entry = Arc::new(Mutex::new(db::Entry::new(key.into(), value.into())));
        data_map.insert(key.into(), Arc::clone(&entry));
        // do not hold the lock for nothing, as the next operations do not need it.
        drop(data_map);
        self.size.fetch_add(1, Ordering::SeqCst);
        entry.clone()
    }

    fn remove(&mut self, key: &str) -> Option<Arc<Mutex<db::Entry>>> {
        let mut data_map = self.data.write().unwrap();
        let entry = data_map.remove(key);
        // do not hold the lock for nothing, as the next operations do not need it.
        drop(data_map);
        self.size.fetch_sub(1, Ordering::SeqCst);
        entry
    }
}
