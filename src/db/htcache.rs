use crate::db;
use crate::db::{EvictionFn, EvictionPolicy, HTPage};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct HTCache {
    pages: Vec<Arc<Mutex<HTPage>>>,
    num_pages: usize,
    num_entries_per_page: usize,
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

        // assumes n_pages and n_entries are powers of 2
        let page_bits = self.num_pages.trailing_zeros();
        let entry_bits = self.num_entries_per_page.trailing_zeros();

        let page_mask = (1 << page_bits) - 1;
        let entry_mask = (1 << entry_bits) - 1;

        // Bitwise AND with masks extracts bits
        let page_location = key_hash & page_mask;
        let entry_location = (key_hash >> page_bits) & entry_mask;

        (page_location, entry_location)
    }

    /// new uses base 2 to compute number of pages and number of entries per page
    pub fn new(page_space: u32, entry_space: u32, eviction_policy: EvictionPolicy) -> Self {
        let num_pages = 2_usize.pow(page_space);
        let num_entries_per_page = 2_usize.pow(entry_space);
        let mut pages = Vec::with_capacity(num_pages);

        let eviction_function = db::get_choose_evict_fn(eviction_policy);

        for _ in 0..num_pages {
            let page = HTPage::new(num_entries_per_page, eviction_function);
            pages.push(Arc::new(Mutex::new(page)));
        }

        HTCache {
            pages,
            num_pages,
            num_entries_per_page,
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

        page.lock().unwrap().get_value(entry_num as usize, key)
    }

    pub fn set_kv(&self, key: &str, value: &str) {
        let (page_num, entry_num) = self.get_entry_location(key);
        // get the relevant page to update
        let result = self.pages.get(page_num as usize);

        if let Some(page) = result {
            // mutex should not be poisoned so it is ok to panic if it happen to
            // show there is a bug in our program.
            let mut guard_page = page.lock().unwrap();
            guard_page.insert_or_push_kv(entry_num as usize, key, value);
        } else {
            // not normal, this should not happen
            eprintln!("the page number {} does not exist", page_num)
        }
    }

    pub fn delete_entries(&self, keys: &Vec<String>) -> usize {
        let keys_set: HashSet<_> = keys.iter().collect();
        keys_set
            .iter()
            .map(|key| self.delete_key_entries(key))
            .sum()
    }

    fn delete_key_entries(&self, key: &&String) -> usize {
        let location = self.get_entry_location(key);
        let (page_number, entry_index) = location;

        let mut page = self
            .pages
            .get(page_number as usize)
            .unwrap()
            .lock()
            .unwrap();

        // @TODO: check if we need to start from the beginning because maybe the eviction could insert some keys
        // anywhere. To check.
        for entry_option in page.entries.iter_mut().skip(entry_index as usize) {
            if let Some(entry) = entry_option {
                if entry.key == **key {
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
    use std::sync::Barrier;
    use std::thread;

    fn generate_gets(htcache: Arc<HTCache>, num: usize) {
        for i in 0..num {
            let key = format!("key{}", i);
            htcache.get_value_for_key(&key);
        }
    }

    fn generate_sets(htcache: Arc<HTCache>, num: usize) {
        for i in 0..num {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            htcache.set_kv(&key, &value);
        }
    }

    fn generate_dels(htcache: Arc<HTCache>, num: usize) {
        for i in 0..num {
            let key = format!("key{}", i);
            htcache.delete_entries(&vec![key]);
        }
    }

    #[test]
    fn test_set_get_del() {
        let cache = HTCache::new(4, 4, EvictionPolicy::RANDOM);
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
        let deleted_num = cache.delete_entries(&vec!["key1".into()]);
        assert_eq!(deleted_num, 1, "one key should be deleted");
        let deleted_num = cache.delete_entries(&vec!["key1".into()]);
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

    #[test]
    fn test_cache_concurrent() {
        let num_threads = 70;
        let cache = Arc::new(HTCache::new(16, 8, EvictionPolicy::RANDOM));
        let barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = vec![];

        for _ in 0..50 {
            let cache = Arc::clone(&cache);
            let barrier = Arc::clone(&barrier);
            let handle = thread::spawn(move || {
                barrier.wait();
                generate_sets(cache, 5000000);
            });
            handles.push(handle);
        }

        for _ in 0..10 {
            let cache = Arc::clone(&cache);
            let barrier = Arc::clone(&barrier);
            let handle = thread::spawn(move || {
                barrier.wait();
                generate_gets(cache, 30000);
            });
            handles.push(handle);
        }

        for _ in 0..10 {
            let cache = Arc::clone(&cache);
            let barrier = Arc::clone(&barrier);
            let handle = thread::spawn(move || {
                barrier.wait();
                generate_dels(cache, 10000);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
