use crate::db::Entry;
use std::collections::{HashMap, LinkedList};
use std::sync;

pub struct LRUCache {
    data: sync::RwLock<HashMap<String, Entry>>,
    max_size: usize,
    list: sync::RwLock<LinkedList<Entry>>,
}
