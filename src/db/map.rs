use crate::db;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

/// Map is a std::collection::HashMap based storage for the cache service.
struct Map {
    max_items: usize,
    size: AtomicUsize,
    // arc because we need to share the reference of the same entry with the eviction struct.
    data: RwLock<HashMap<String, Arc<db::Entry>>>,
}

impl db::Storage for Map {
    fn is_full(&self) -> bool {
        self.size.load(Ordering::SeqCst) >= self.max_items
    }

    fn get(&self, key: &str) -> Option<Arc<db::Entry>> {
        // Calling unwrap here is Ok because we want to panic is the mutex gets poisoned.
        // It is the right thing to do.
        let guard = self.data.read().unwrap();
        if let Some(entry) = guard.get(key) {
            return Some(entry.clone());
        }
        None
    }

    fn set(&mut self, key: &str, value: &str) -> Arc<db::Entry> {
        let mut guard = self.data.write().unwrap();
        let entry = Arc::new(db::Entry::new(key.into(), value.into()));
        guard.insert(key.into(), entry);
        // do not hold the lock for nothing, as the next operations do not need it.
        drop(guard);
        self.size.fetch_add(1, Ordering::SeqCst);
        entry.clone()
    }

    fn remove(&mut self, key: &str) -> Option<Arc<db::Entry>> {
        let mut guard = self.data.write().unwrap();
        let entry = guard.remove(key);
        // do not hold the lock for nothing, as the next operations do not need it.
        drop(guard);
        self.size.fetch_sub(1, Ordering::SeqCst);
        entry
    }
}
