use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    tx: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    pub fn new(n: usize) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));
        let workers = (0..n)
            .map(|_| {
                let rx = Arc::clone(&rx);
                thread::spawn(move || loop {
                    // The lock is held only while waiting for the next job.
                    let job = rx.lock().unwrap().recv();
                    match job {
                        Ok(job) => job(),
                        Err(_) => break, // sender dropped: shut down
                    }
                })
            })
            .collect();
        ThreadPool { workers, tx: Some(tx) }
    }

    pub fn execute<F: FnOnce() + Send + 'static>(&self, f: F) {
        self.tx.as_ref().unwrap().send(Box::new(f)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.tx.take()); // closes the channel
        for w in self.workers.drain(..) {
            w.join().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[test]
    fn runs_all_jobs() {
        let hits = Arc::new(AtomicUsize::new(0));
        {
            let pool = ThreadPool::new(4);
            for _ in 0..100 {
                let hits = hits.clone();
                pool.execute(move || { hits.fetch_add(1, Ordering::Relaxed); });
            }
        } // drop joins workers
        assert_eq!(hits.load(Ordering::Relaxed), 100);
    }
}
