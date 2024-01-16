use crate::db::EvictionFn;
use crate::db::HTPageEntry;
use std::sync::{RwLock, RwLockWriteGuard};

pub struct HTPage {
    pub entries: Vec<Option<RwLock<HTPageEntry>>>,
    evict_and_replace: EvictionFn,
}

impl HTPage {
    pub fn new(size: usize, eviction_function: EvictionFn) -> Self {
        let mut entries: Vec<Option<RwLock<HTPageEntry>>> = Vec::with_capacity(size);
        for _ in 0..size {
            entries.push(None);
        }
        HTPage {
            entries,
            evict_and_replace: eviction_function,
        }
    }

    pub fn get_value(&self, index: usize, key: &str) -> Option<String> {
        self.entries
            .iter()
            .skip(index)
            .find(|&x| x.is_some() && x.as_ref().unwrap().read().unwrap().key == key)
            .and_then(|x| {
                let mut write_entry = x.as_ref()?.write().ok()?;
                write_entry.freq_count += 1;
                Some(write_entry.value.clone())
            })
    }

    /// insert_or_push_kv inserts the entry the location provided if it is empty.
    /// If this location is not empty, it means collision occurred so chain the entry (Closed addressing collision resolution).
    ///
    pub fn insert_or_push_kv(&mut self, index: usize, key: &str, value: &str) {
        if let Some(entry) = self.entries[index].as_ref() {
            // entry exist, so update it
            let mut entry = entry.write().unwrap();
            Self::update_entry(&mut entry, key, value);
            return;
        }

        if !Self::probe_for_empty_slot(&mut self.entries, index, key, value) {
            // handle eviction
            (self.evict_and_replace)(&mut self.entries, key, value);
        }
    }

    fn update_entry(entry: &mut RwLockWriteGuard<HTPageEntry>, key: &str, value: &str) {
        if entry.key == key {
            entry.value = value.into();
            entry.freq_count += 1;
        }
    }

    fn probe_for_empty_slot(
        entries: &mut [Option<RwLock<HTPageEntry>>],
        index: usize,
        key: &str,
        value: &str,
    ) -> bool {
        if let Some(x) = entries.iter_mut().skip(index).find(|x| x.is_none()) {
            let new_entry = HTPageEntry::new(key, value, None);
            x.replace(RwLock::new(new_entry));
            return true;
        }
        false
    }
}
