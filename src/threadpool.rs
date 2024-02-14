use std::fmt::{Debug, Formatter};
use std::{
    io, panic,
    sync::{mpsc, Arc, Mutex},
    thread,
};
use tracing::{debug, error};

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
    size: usize,
}

impl Debug for ThreadPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ThreadPool: {{workers: {}}}", self.size)
    }
}

impl ThreadPool {
    /// Creates a new thread pool with the specified number of workers.
    ///
    /// # Arguments
    ///
    /// * `size` - The number of workers to create in the thread pool.
    ///
    /// # Panics
    ///
    /// This function will panic if `size` is zero or if no thread could
    /// be created by the Operating System.
    ///
    /// # Returns
    ///
    /// A new `ThreadPool` instance with the specified number of workers.
    pub fn new(size: usize) -> io::Result<ThreadPool> {
        if size == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot create a thread pool with zero workers",
            ));
        }
        let (sender, receiver) = create_shared_channel();

        let mut workers = Vec::with_capacity(size);
        for i in 0..size {
            workers.push(Worker::new(i, receiver.clone())?);
            debug!(worker_id = i, "worker created");
        }

        Ok(ThreadPool {
            workers,
            sender,
            size,
        })
    }

    /// Executes the given closure `f` on a thread in the thread pool.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure to be executed on a thread in the thread pool.
    ///
    /// # Example
    ///
    /// ```
    /// //use redisy::threadpool::ThreadPool;
    ///
    /// //let thread_pool = ThreadPool::new(4).unwrap();
    ///
    /// //thread_pool.execute(|| {
    /// //    println!("This closure is executed on a thread in the thread pool");
    /// //});
    /// ```
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Message::Task(Box::new(f));
        match self.sender.send(job) {
            Ok(_) => {}
            Err(e) => error!("fail sending job to worker: {}", e),
        };
    }

    /// shutdown Shuts down the thread pool. It sends a `Message::Shutdown` to each worker and waits for them to finish.
    /// It also decrements the `size` of the thread pool as each worker thread finishes to avoid using a dropped `sender`
    /// in subsequent calls to `shutdown`.
    pub fn shutdown(&mut self) {
        for _ in 0..self.size {
            if let Err(e) = self.sender.send(Message::Shutdown) {
                error!("error sending Shutdown cmd: {}", e);
            }
        }
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread
                    .join()
                    .unwrap_or_else(|_| error!("error while joining thread"));
                // At the end of the execution of this method, all the workers will be replaced
                // by None and the `sender` end of the channel will be dropped. So running this method
                // again would make an attempt to use a dropped `sender`. To avoid that, we
                // decrement `size` as the thread shutdown. This way, subsequent calls to shut down
                // would have no effect.
                self.size -= 1
            }
        }
    }
}

impl Drop for ThreadPool {
    /// If a ThreadPool goes out of scope, it would drop the channel at the sender end.
    /// Dropping at this end will cause the connection to drop so some jobs might not reach execution.
    /// The programmer is supposed to call the `shutdown` method by himself but in case he does not, the drop method
    /// would hold his back by joining the threads.
    /// Doing so will allow all the jobs to finish and the threads to gracefully exit.
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// `Worker` is a struct that represents a worker thread. Each worker has a unique identifier assigned via the `id` field.
#[derive(Debug)]
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// Creates a new worker thread with the given id and shared receiver.
    ///
    /// # Arguments
    ///
    /// * `id` - An identifier for the worker thread.
    /// * `receiver` - A shared receiver for the worker thread to receive messages from.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Worker` if the thread was successfully created, or `Error` if the OS failed to create the thread.
    fn new(id: usize, receiver: SharedReceiver) -> io::Result<Worker> {
        let worker_process = move || Self::process_messages(id, &receiver);
        let thread = thread::Builder::new().spawn(worker_process)?;
        Ok(Worker {
            id,
            thread: Some(thread),
        })
    }

    fn process_messages(id: usize, receiver: &SharedReceiver) {
        loop {
            match receiver.get_message() {
                Message::Task(job) => {
                    debug!("worker {} received a job", id);
                    let result = panic::catch_unwind(panic::AssertUnwindSafe(job));

                    if result.is_err() {
                        error!("the job caused the worker {} to panic!", id);
                    }
                }

                Message::Shutdown => {
                    debug!("graceful shutdown from worker {}", id);
                    break;
                }

                Message::Error(e) => {
                    error!("job read error occurred from worker {}: {}", id, e);
                }
            }
        }
    }
}

#[derive(Clone)]
struct SharedReceiver {
    receiver: Arc<Mutex<mpsc::Receiver<Message>>>,
}

impl SharedReceiver {
    /// Retrieves a message from the receiver channel.
    ///
    /// # Returns
    ///
    /// Returns a `Message` enum variant that contains either a `Task` or an `Error` message.
    ///
    fn get_message(&self) -> Message {
        let mutex_guard = self.receiver.lock().unwrap();
        mutex_guard
            .recv()
            .unwrap_or_else(|e| Message::Error(e.to_string()))
    }
}
impl Iterator for SharedReceiver {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        let guard = self.receiver.lock().unwrap();
        guard.recv().ok()
    }
}

/// Creates a shared channel for communication between threads.
/// Returns a tuple containing a sender and a shared receiver.
fn create_shared_channel() -> (mpsc::Sender<Message>, SharedReceiver) {
    let (sender, receiver) = mpsc::channel();
    (
        sender,
        SharedReceiver {
            receiver: Arc::new(Mutex::new(receiver)),
        },
    )
}

/// A type alias for a job to be executed by the thread pool.
type Job = Box<dyn FnOnce() + 'static + Send>;

/// `Message` represents work which will be shared to the worker threads. We use enum to easily
/// distinguish between jobs and shutdown instruction. Note for learning purpose: this could also
/// be achieved using an atomic bool shared to all the threads.
enum Message {
    /// A worker receiving this message variant has to shut down (break the infinite loop)
    Shutdown,
    /// `Job` represents a job to be executed by a worker
    Task(Job),
    /// Failing to read messages from the shared channel should not error.
    /// This is why we define an Error message variant which will be shared to the thread in case we get a channel
    /// receive error or a mutex lock error (poisoned or blocking).
    Error(String),
}
