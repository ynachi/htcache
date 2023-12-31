mod lfu;

mod lru;

// reexport. This allow public fields to be used directly from eviction.
// example use crate::db::eviction::lru_public_fn;
use crate::db;
pub use lfu::*;

//
// Eviction factory method.
// New types should also be added here.
pub enum EvictionPolicyType {
    // LRU,
    LFU,
    // TTL,
}

pub fn create_eviction_policy(eviction_policy_type: EvictionPolicyType) -> impl db::Eviction {
    match eviction_policy_type {
        EvictionPolicyType::LFU => LFU::new(),
    }
}
