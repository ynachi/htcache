use crate::db;
use crate::db::{EvictionFn, EvictionPolicy, HTPage};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

pub struct HTCache {
    pages: Vec<RwLock<HTPage>>,
    // number of pages
    n_pages: usize,
    // number of entries per page
    n_entries: usize,
    // eviction policy
    eviction_function: EvictionFn,
}

impl HTCache {
    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        hasher.finish()
    }

    fn get_entry_location(&self, key: &str) -> (u64, u64) {
        let key_hash = Self::calculate_hash(&key);
        let page_location = key_hash % self.n_pages as u64;
        let entry_location = (key_hash / self.n_pages as u64) % self.n_entries as u64;
        (page_location, entry_location)
    }

    pub fn new(n_pages: usize, n_entries: usize, eviction_policy: EvictionPolicy) -> Self {
        let mut pages: Vec<RwLock<HTPage>> = Vec::with_capacity(n_pages);

        let eviction_function = db::get_choose_evict_fn(eviction_policy);

        for _ in 0..n_pages {
            let page = HTPage::new(n_entries, eviction_function);
            pages.push(RwLock::new(page));
        }

        HTCache {
            pages,
            n_pages,
            n_entries,
            eviction_function,
        }
    }

    pub fn get_value_for_key(&self, key: &str) -> Option<String> {
        let location = self.get_entry_location(key);
        let (page_num, entry_num) = location;

        // We do not check if option is none here because this should not happen if our program is correct.
        // So it is ok to panic as this is clearly a bug.
        let page = self
            .pages
            .get(page_num as usize)
            .expect("this is clearly a bug as page {} should normally exist");

        page.read().unwrap().get_value(entry_num as usize, key)
    }

    pub fn set_kv(&mut self, key: &str, value: &str) {
        let (page_num, entry_num) = self.get_entry_location(key);
        // get the relevant page to update
        let result = self.pages.get(page_num as usize);

        if let Some(page) = result {
            // mutex should not be poisoned so it is ok to panic if it happen to
            // show there is a bug in our program.
            let mut write_guard_page = page.write().unwrap();
            write_guard_page.insert_or_push_kv(entry_num as usize, key, value);
        } else {
            // not normal, this should not happen
            eprintln!("the page number {} does not exist", page_num)
        }
    }

    pub fn delete_entries(&mut self, keys: &[&str]) -> usize {
        let keys_set: HashSet<_> = keys.iter().collect();
        keys_set
            .iter()
            .map(|key| self.delete_key_entries(key))
            .sum()
    }

    fn delete_key_entries(&mut self, key: &&str) -> usize {
        let location = self.get_entry_location(key);
        let (page_number, entry_index) = location;

        let mut page = self
            .pages
            .get(page_number as usize)
            .unwrap()
            .write()
            .unwrap();

        // @TODO: check if we need to start from the beginning because maybe the eviction could insert some keys
        // anywhere. To check.
        for entry_option in page.entries.iter_mut().skip(entry_index as usize) {
            if let Some(entry) = entry_option {
                if entry.read().unwrap().key == **key {
                    *entry_option = None;
                    // exit here as every key should be uniq in a page
                    return 1;
                }
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_get_del() {
        let mut cache = HTCache::new(16, 4, EvictionPolicy::RANDOM);
        for i in 0..10 {
            cache.set_kv(&format!("key{}", i), &format!("value{}", i))
        }

        // test get key found
        for i in 0..10 {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            assert_eq!(
                cache.get_value_for_key(&key).unwrap(),
                value,
                "key should exist in the cache"
            );
        }

        // test get key not found
        assert_eq!(
            cache.get_value_for_key("not found"),
            None,
            "key should not exist in the cache"
        );

        // test delete key
        let deleted_num = cache.delete_entries(&["key1"]);
        assert_eq!(deleted_num, 1, "one key should be deleted");
        let deleted_num = cache.delete_entries(&["key1"]);
        assert_eq!(
            deleted_num, 0,
            "0 key should be deleted as it was deleted before"
        );
        assert_eq!(
            cache.get_value_for_key("key1"),
            None,
            "key1 should no longer exist as it was deleted before"
        );

        // update key2 and check its new value. We try to update time than the cache contains items.
        for _ in 0..100 {
            cache.set_kv("key2", "value100");
        }
        assert_eq!(
            cache.get_value_for_key("key2").unwrap(),
            "value100",
            "key2 value should have changed to value100"
        );

        // lets force eviction, it should not panic.
        for i in 0..100 {
            cache.set_kv(&format!("key{}", i), &format!("value{}", i))
        }
    }
}
