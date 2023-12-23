use crate::db::{Database, Entry};
use crate::error;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync;

struct LFUCache {
    data: sync::RwLock<HashMap<String, Entry>>,
    max_size: usize,
    rank: sync::RwLock<BinaryHeap<Reverse<Entry>>>,
}

impl LFUCache {
    fn is_full(&self) -> Result<bool, error::DatabaseError> {
        let curr_size = self.get_size()?;
        Ok(curr_size >= self.max_size)
    }

    fn get_size(&self) -> Result<usize, error::DatabaseError> {
        let data = self
            .data
            .read()
            .map_err(|_| error::DatabaseError::PoisonedMutex)?;
        Ok(data.len())
    }
}

impl Database for LFUCache {
    fn get(&mut self, key: &str) -> Result<Option<Entry>, error::DatabaseError> {
        let data = self
            .data
            .read()
            .map_err(|_| error::DatabaseError::PoisonedMutex)?;
        if let Some(entry) = data.get(key) {
            let mut entry = entry.clone();
            entry.update_frequency();
            return Ok(Some(entry.clone()));
        }
        Ok(None)
    }

    fn set(&mut self, key: String, entry: Entry) -> Result<(), error::DatabaseError> {
        // need to evict ?
        while self.is_full()? {
            let to_evict_key = self.chose_evict()?;
            if let Some(to_evict_key) = to_evict_key {
                self.delete(to_evict_key)?;
            }
        }
        // insert the key
        let mut data = self
            .data
            .write()
            .map_err(|_| error::DatabaseError::PoisonedMutex)?;
        data.insert(key, entry.clone());
        // now refresh
        let mut rank = self
            .rank
            .write()
            .map_err(|_| error::DatabaseError::PoisonedMutex)?;
        rank.push(Reverse(entry));
        Ok(())
    }

    fn delete(&mut self, key: String) -> Result<(), error::DatabaseError> {
        let mut data = self
            .data
            .write()
            .map_err(|_| error::DatabaseError::PoisonedMutex)?;
        data.remove(&key);
        // no need to also remove in the heap. The cleanup will be done by itself as we evict keys
        Ok(())
    }

    fn iterate(&self) -> Box<dyn Iterator<Item = Entry>> {
        todo!()
    }

    fn chose_evict(&self) -> Result<Option<String>, error::DatabaseError> {
        let mut rank = self
            .rank
            .write()
            .map_err(|_| error::DatabaseError::PoisonedMutex)?;
        if let Some(entry) = rank.pop() {
            return Ok(Some(entry.0.key));
        }
        Ok(None)
    }
}
