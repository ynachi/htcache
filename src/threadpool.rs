use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

/// `ThreadPool` is a data structure representing a pool of threads which continuously watch
/// for new jobs to execute until they are explicitly shutdown. This struct is not meant to be
/// instantiated directly. It is instead created using the `new` method. There is no explicit
/// worker pool termination. The threads terminates when the pool goes out of scope. (see
/// drop trait implementation for `ThreadPool`). A `ThreadPool` should ne terminated by calling
/// the `shutdown` method. Not doing so will cause the program to panic. This was a design
/// choice to allow the programmer to explicitly shutdown a `ThreadPool` when needed.
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
    size: usize
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        if size == 0 {
            panic!("cannot create a thread pool with zero workers");
        }
        let (sender, receiver) = create_shared_channel();

        let mut workers = Vec::with_capacity(size);
        for i in 0..size {
            if let Some(worker) = Worker::new(i, receiver.clone()) {
                workers.push(worker);
            }
        }

        // A pool without any worker cannot be used, so panic is this case
        assert!(!workers.is_empty());

        ThreadPool {
            workers,
            sender,
            size
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Message::Task(Box::new(f));
        // @TODO Manage error
        self.sender.send(job).unwrap();
    }

    pub fn shutdown(&mut self) {
        for _ in 0..self.size {
            // @TODO Manage error
            self.sender.send(Message::Shutdown).unwrap();
        }
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
                // At the end of the execution of this method, all the workers will be replace
                // by None and the `sender` end of the channel will be drop. So running this method
                // again would make an attempt to use a dropped `sender`. To avoid that, we
                // decrement `size` as the thread shutdown. This way, subsequent calls to shutdown
                // would have no effect.
                self.size -= 1
            }
        }
    }
}

impl Drop for ThreadPool {
    /// If a ThreadPool goes out of scope, it would drop the channel sender end. Dropping this end
    /// will cause the connexion to drop so some jobs might not reach execution. The programmer is
    /// suppose to call the `shutdown` method by himself but in case he does not, the drop method
    /// would hold his back by joining the threads. Doing so will allow all the jobs to finish and
    /// the threads to gracefully exit.
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// `Worker` is a struct that represents a worker thread. Each worker has a unique identifier assigned via the `id` field.
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: SharedReceiver) -> Option<Worker> {
        // @TODO, this will panic if the OS cannot create a thread for some reasons.
        // fix it using thread builder
        let builder = thread::Builder::new();

        let thread = builder.spawn(move || {
            loop {
                // @TODO Manager errors
                match receiver.get_message() {
                    Message::Task(job) => {
                        // @TODO Manage logging
                        println!("worker {} received a job", id);
                        job();
                    }
                    Message::Shutdown => {
                        // @TODO Manage logging
                        println!("Graceful shutdown from worker {}", id);
                        break;
                    }
                    Message::Error(e) => {
                        // @TODO Manage logging
                        println!("job read error occurred from worker {}: {}", id, e);
                    }
                }
            }
        });

            match thread {
                Ok(thread) => Some(Worker {
                    id,
                    thread: Some(thread),
                }),
                Err(e) => {
                    println!("os failed to create thread {}: {}", id, e);
                    None
                }
            }
    }
}

#[derive(Clone)]
struct SharedReceiver {
    receiver: Arc<Mutex<mpsc::Receiver<Message>>>,
}

impl SharedReceiver {
    fn get_message(&self) -> Message {
        let mutex_guard = self.receiver.lock();
        match mutex_guard {
            Ok(mutex_guard) => match mutex_guard.recv() {
                Ok(message) => message,
                Err(e) => Message::Error(e.to_string()),
            },
            Err(e) => Message::Error(e.to_string()),
        }
    }
}
impl Iterator for SharedReceiver {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        // @TODO Manage errors
        let guard = self.receiver.lock().unwrap();
        guard.recv().ok()
    }
}

fn create_shared_channel() -> (mpsc::Sender<Message>, SharedReceiver) {
    let (sender, receiver) = mpsc::channel();
    (
        sender,
        SharedReceiver {
            receiver: Arc::new(Mutex::new(receiver)),
        },
    )
}

type Job = Box<dyn FnOnce() + 'static + Send>;

/// `Message` represents work which will be shared to the worker threads. We use enum to easily
/// distinguish between jobs and shutdown instruction. Note for learning purpose: this could also
/// be achieved using an atomic bool shared to all the threads.
enum Message {
    /// A worker receiving this message variant has to shutdown (break the infinite loop)
    Shutdown,
    /// `Job` represents a job to be executed by a worker
    Task(Job),
    /// Failing to read messages from the shared should not error. This is why we define an Error
    /// message variant which will ne shared to the thread in case we get a channel receive error
    /// or a mutex lock error (poisoned or blocking).
    Error(String),
}
