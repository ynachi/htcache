// We need to move on and learn other things, so here is the final design
// We will evict on ttl + random.
// A background job would evict the cache when the capacity reaches a certain threshold.
//If you try to set an element and there is no space, random eviction will happen.

extern crate rand;
use crate::db::cmap::CMap;
use metrics::{counter, describe_counter};
use std::collections::BTreeSet;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::{io, thread};
use tracing::debug;

const METRIC_EVICTED_KEY: &str = "evicted_keys";
const METRIC_EVICTED_KEY_DESC: &str = "number of evicted keys";
const LABEL_EVICTED_KEY_SHARD: &str = "shard";

pub struct Cache {
    // We could have put everything in the same struct,
    // but this means
    // this struc could need to be shared to all threads while only State is required.
    // Cache will behave like a higher level struct orchestrating state sharing among threads.
    storage: Arc<State>,
    // Cleanup_needed will signal the background thread to start cleaning up.
    // So it needs to be created here and shared to State.
    cleanup_needed: Arc<(Mutex<bool>, Condvar)>,
    // The cleanup background job will run in a thread.
    // We don't have mechanism to shut down this job.
    // So the handler is owned by the Cache structure.
    // This way, the job will be dropped when the cache goes out of scope.
    cleanup_job: JoinHandle<()>,
}

impl Cache {
    pub fn db(&self) -> Arc<State> {
        self.storage.clone()
    }

    pub fn create_cleanup_job(
        cleanup_needed: Arc<(Mutex<bool>, Condvar)>,
        state: Arc<State>,
    ) -> io::Result<JoinHandle<()>> {
        thread::Builder::new()
            .name("htcache-eviction-job".to_string())
            .spawn(move || {
                loop {
                    let (lock, cvar) = &*cleanup_needed;
                    let mut cleanup_threshold_reached = lock.lock().unwrap();
                    while !*cleanup_threshold_reached {
                        cleanup_threshold_reached = cvar.wait(cleanup_threshold_reached).unwrap();
                    }

                    // We need to perform cleanup here
                    debug!("start performing background automatic eviction");
                    state.evict_expired_keys();
                    debug!("finish performing background automatic eviction");
                    *cleanup_threshold_reached = false;
                }
            })
    }
}

impl Debug for Cache {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cache: {{state: {:?}}}", self.storage)
    }
}

pub fn create_cache(
    capacity: usize,
    shard_count: usize,
    auto_eviction_threshold: u8,
) -> io::Result<Cache> {
    if auto_eviction_threshold >= 100 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "eviction threshold should be between 0 and 99",
        ));
    }

    let cleanup_needed = Arc::new((Mutex::new(false), Condvar::new()));
    let cleanup_needed_clone = cleanup_needed.clone();
    let state = Arc::new(State::new(
        capacity,
        shard_count,
        cleanup_needed_clone,
        auto_eviction_threshold,
    )?);

    let job = Cache::create_cleanup_job(cleanup_needed.clone(), state.clone())
        .expect("failed to create cleanup background job");

    Ok(Cache {
        storage: state,
        cleanup_needed: cleanup_needed.clone(),
        cleanup_job: job,
    })
}

/// State is the underlined data structure of the Cache.
pub struct State {
    // Data is the core storage. It is based on a concurrent, bucket-based hashmap data structure.
    // Cmap uses read-write lock at bucket level.
    // This allows parallel access to read and write the state.
    data: CMap,
    // The total number of elements in the cache. We do not want it to grow indefinitely.
    // Capacity should be a power of two.
    capacity: usize,
    // Expired keys are not immediately evicted.
    // We instead rely on a background job which runs when the storage is at a certain capacity.
    // There is no systematic eviction, so it means
    // that the capacity is not guaranteed to be respected.
    // If the keys do not expire often and new ones keep being added,
    // capacity would outgrow the set value.
    // auto_eviction_threshold: u8,
    // Keeps track of items expiration order.
    // Using a HashSet will rank items per expiration
    // so the background job does not have
    // to loop over all of them to find which one needs to be evicted.
    tracking: Mutex<BTreeSet<(Instant, String)>>,
    // Expired keys are not immediately evicted.
    // We instead rely on a background job which runs when the storage is at a certain capacity.
    // There is no systematic eviction, so it means
    // that the capacity is not guaranteed to be respected.
    // If the keys do not expire often and new ones keep being added,
    // capacity would outgrow the set value.
    auto_eviction_threshold: u8,
    // shared cleanup flag with the parent struct Cache.
    cleanup_needed: Arc<(Mutex<bool>, Condvar)>,
    shard_count: usize,
}

impl State {
    pub fn new(
        capacity: usize,
        shard_count: usize,
        cleanup_needed: Arc<(Mutex<bool>, Condvar)>,
        auto_eviction_threshold: u8,
    ) -> io::Result<Self> {
        let data = CMap::new(shard_count, capacity / shard_count)?;
        let tracking = Mutex::new(BTreeSet::new());
        Ok(Self {
            data,
            capacity,
            tracking,
            auto_eviction_threshold,
            cleanup_needed,
            shard_count,
        })
    }

    fn evict_expired_keys(&self) {
        // get keys that need to be deleted and remove them from tracking
        let mut guard = self.tracking.lock().unwrap();
        let cut_off_item = Self::get_last_item_before_cutoff(&guard);
        let expired_items = guard.split_off(&cut_off_item);
        // now remove them from the primary storage
        let mut keys = Vec::new();
        for item in expired_items {
            keys.push(item.1);
        }
        let evicted = self.data.del_entries(&keys);
        // emit metrics
        describe_counter!(METRIC_EVICTED_KEY, METRIC_EVICTED_KEY_DESC);
        for (shard_id, count) in evicted {
            counter!(METRIC_EVICTED_KEY, LABEL_EVICTED_KEY_SHARD => shard_id.to_string())
                .increment(count as u64);
        }
        // todo!("can we set metric description only once in main?");
    }

    fn get_last_item_before_cutoff(
        instants: &MutexGuard<BTreeSet<(Instant, String)>>,
    ) -> (Instant, String) {
        let cut_off_instant = Instant::now();
        let mut prev_item = (Instant::now(), String::new());
        for item in instants.iter() {
            if item.0 >= cut_off_instant {
                break;
            }
            prev_item = item.clone();
        }
        prev_item
    }

    pub fn set_kv(&self, key: &str, value: &str, ttl: Option<Duration>) {
        // Insert
        // let expiration_time = if let Some(ttl) = ttl {
        //     Instant::now() + ttl
        // } else {
        //     Instant::now()
        // };
        // let entry = CacheEntry::new(key, value, expiration_time);
        self.data.set_kv(key, value);

        let current_size = self.data.size();

        // check if global eviction is needed
        if current_size >= (self.capacity * self.auto_eviction_threshold as usize / 100) {
            let (lock, cvar) = &*self.cleanup_needed;
            let mut cleanup_threshold_reached = lock.lock().unwrap();
            *cleanup_threshold_reached = true;
            // notify background eviction thread
            cvar.notify_one();
            debug!(
                "automatic eviction thread notified, current_size: {}",
                current_size
            );
        }

        // // Track key
        // self.tracking
        //     .lock()
        //     .unwrap()
        //     .insert((expiration_time, key.to_string()));
    }

    pub fn get_value_by_key(&self, key: &str) -> Option<String> {
        self.data.get_value(key)
    }

    pub fn delete_entries(&self, keys: &Vec<String>) -> usize {
        let deleted_items = self.data.del_entries(keys);
        let mut ans = 0;
        for (_, v) in deleted_items {
            ans += v;
        }
        ans
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "State{{capacity: {}, shards: {}}}",
            self.capacity, self.shard_count
        )
    }
}
