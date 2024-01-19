use crate::db::EvictionFn;
use crate::db::HTPageEntry;
use std::sync::{RwLock, RwLockWriteGuard};

pub struct HTPage {
    pub entries: Vec<Option<HTPageEntry>>,
    evict_and_replace: EvictionFn,
}

impl HTPage {
    pub fn new(size: usize, eviction_function: EvictionFn) -> Self {
        let mut entries = Vec::with_capacity(size);
        for _ in 0..size {
            entries.push(None);
        }
        HTPage {
            entries,
            evict_and_replace: eviction_function,
        }
    }

    pub fn get_value(&mut self, index: usize, key: &str) -> Option<String> {
        self.entries
            .iter_mut()
            .skip(index)
            .filter_map(|entry_option| match entry_option {
                Some(entry) if entry.key == key => {
                    entry.increment_frequency();
                    Some(entry.value.clone())
                }
                _ => None,
            })
            .next()
    }

    /// insert_or_push_kv inserts the entry the location provided if it is empty.
    /// If this location is not empty, it means collision occurred so chain the entry (Closed addressing collision resolution).
    ///
    pub fn insert_or_push_kv(&mut self, index: usize, key: &str, value: &str) {
        if let Some(entry) = self.entries[index].as_mut() {
            // entry exist, so update it
            if entry.key == key {
                entry.update_fields(value);
            }
            return;
        }

        if !Self::probe_for_empty_slot(&mut self.entries, index, key, value) {
            // handle eviction
            (self.evict_and_replace)(&mut self.entries, key, value);
        }
    }

    fn probe_for_empty_slot(
        entries: &mut [Option<HTPageEntry>],
        index: usize,
        key: &str,
        value: &str,
    ) -> bool {
        if let Some(x) = entries.iter_mut().skip(index).find(|x| x.is_none()) {
            let new_entry = HTPageEntry::new(key, value, None);
            x.replace(new_entry);
            return true;
        }
        false
    }
}
