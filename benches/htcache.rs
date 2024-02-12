use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dashmap::DashMap;
use htcache::db::cmap::CMap;

use rand::distributions::{Alphanumeric, DistString};

use rayon::prelude::*; // For threading
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use csv::ReaderBuilder;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::vec::Vec;

pub fn read_csv_file() -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let file_path = Path::new("/Users/ynachi/codes/github.com/htcache/testdata/surnames.csv");
    let file = File::open(file_path)?;
    let mut rdr = ReaderBuilder::new().delimiter(b',').from_reader(file);
    let mut records = vec![];
    for result in rdr.records() {
        let record = result?;
        // let entry = db::CacheEntry::new(&record[0], &record[2], Instant::now());
        records.push((record[0].to_string(), record[2].to_string()));
    }
    Ok(records)
}

pub fn generate_string() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 30)
}

fn generate_test_kp(number_of_items: usize) -> Vec<(String, String)> {
    let mut ans = Vec::new();
    for _ in 0..number_of_items {
        let key = generate_string();
        let value = generate_string();
        ans.push((key.clone(), value));
    }
    ans
}

fn cmap_write(test_data: &Vec<(String, String)>) -> Arc<CMap> {
    let test_size = test_data.len(); // Number of entries to insert and retrieve
    let threads = 16; // Example: Using 4 threads, adjust as needed

    // Sharded map
    let sharded_map = Arc::new(CMap::new(32, 500000).unwrap());
    test_data.par_chunks(test_size / threads).for_each(|chunk| {
        let map = Arc::clone(&sharded_map);
        chunk.iter().for_each(|(key, value)| {
            map.set_kv(key, value);
        });
    });
    sharded_map
}

fn cmap_read(test_data: &Vec<(String, String)>) {
    let map = cmap_write(test_data);
    for entry in test_data {
        assert_eq!(&map.get_value(&entry.0).unwrap(), &entry.1);
    }
}
//
fn dash_map_read(test_data: &Vec<(String, String)>) {
    let map = dash_map_write(test_data);
    for entry in test_data {
        assert_eq!(&*map.get(&entry.0).unwrap(), &entry.1);
    }
}

fn dash_map_write(test_data: &Vec<(String, String)>) -> Arc<DashMap<&String, String>> {
    let test_size = test_data.len(); // Nu/ Number of entries to insert and retrieve
    let threads = 16; // Example: Using 4 threads, adjust as needed

    // Sharded map
    let sharded_map = Arc::new(DashMap::with_shard_amount(32));
    test_data.par_chunks(test_size / threads).for_each(|chunk| {
        let map = Arc::clone(&sharded_map);
        chunk.iter().for_each(|(key, value)| {
            map.insert(key, value.clone());
        });
    });
    sharded_map
}

fn regular_map(test_data: &Vec<(String, String)>) {
    let test_size = test_data.len(); // Nu
    let threads = 16; // Example: Using 4 threads, adjust as needed

    // Mutex<HashMap>
    let simple_map = Arc::new(Mutex::new(HashMap::<String, String>::new()));

    test_data.par_chunks(test_size / threads).for_each(|chunk| {
        let map = Arc::clone(&simple_map);
        chunk.iter().for_each(|(key, value)| {
            map.lock().unwrap().insert(key.clone(), value.clone());
        });
    });
}

pub fn criterion_cmap_benchmark(c: &mut Criterion) {
    // let test_data = read_csv_file().unwrap();
    let test_data = read_csv_file().unwrap();
    c.bench_function("cmap-write", |b| {
        b.iter(|| cmap_write(black_box(&test_data)))
    });
    c.bench_function("cmap-read", |b| b.iter(|| cmap_read(black_box(&test_data))));
}

pub fn criterion_regular_map_benchmark(c: &mut Criterion) {
    // let test_data = read_csv_file().unwrap();
    let test_data = generate_test_kp(10000000);
    c.bench_function("hashmap-set", |b| {
        b.iter(|| regular_map(black_box(&test_data)))
    });
    c.bench_function("hashmap-set", |b| {
        b.iter(|| regular_map(black_box(&test_data)))
    });
}

pub fn criterion_dashmap_benchmark(c: &mut Criterion) {
    let test_data = read_csv_file().unwrap();
    // let test_data = generate_test_kp(10000000);
    c.bench_function("dashmap-write", |b| {
        b.iter(|| dash_map_write(black_box(&test_data)))
    });
    c.bench_function("dashmap-read", |b| {
        b.iter(|| dash_map_read(black_box(&test_data)))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(100) // Set your parameters here
        .measurement_time(std::time::Duration::new(60, 800));
    targets = criterion_cmap_benchmark, criterion_dashmap_benchmark
);

criterion_main!(benches);
