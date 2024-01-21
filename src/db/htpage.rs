use crate::db::EvictionFn;
use crate::db::HTPageEntry;

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
        self.find_and_refresh(index, key)
    }

    fn find_and_refresh(&mut self, index: usize, key: &str) -> Option<String> {
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

        if !self.find_and_fill_empty_slot(index, key, value) {
            // handle eviction
            (self.evict_and_replace)(&mut self.entries, key, value);
        }
    }

    fn find_and_fill_empty_slot(&mut self, index: usize, key: &str, value: &str) -> bool {
        if let Some(empty_slot) = self.entries.iter_mut().skip(index).find(|x| x.is_none()) {
            let new_entry = HTPageEntry::new(key, value, None);
            empty_slot.replace(new_entry);
            return true;
        }
        false
    }
}
