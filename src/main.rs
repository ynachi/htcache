use redisy::threadpool::ThreadPool;

fn main() {
    let mut pool = ThreadPool::new(4);
    for i in 0..10 {
        pool.execute(move || println!("Running job id {}", i));
    }
    pool.shutdown();
}
