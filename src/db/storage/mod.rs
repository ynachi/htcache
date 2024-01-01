mod map;
use crate::db::{storage, Storage};
pub use map::*;

//
// Storage factory methods.
// New types should also be added here.
pub enum StorageType {
    Map,
}

pub fn create_storage(storage_type: StorageType, size: usize) -> impl Storage {
    match storage_type {
        StorageType::Map => storage::Map::new(size),
    }
}