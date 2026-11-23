use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

pub struct Bounded<T> {
    q: Mutex<VecDeque<T>>,
    cap: usize,
    not_full: Condvar,
    not_empty: Condvar,
}

impl<T> Bounded<T> {
    pub fn new(cap: usize) -> Self {
        assert!(cap > 0, "a bounded channel requires a positive capacity");
        Bounded {
            q: Mutex::new(VecDeque::with_capacity(cap)),
            cap,
            not_full: Condvar::new(),
            not_empty: Condvar::new(),
        }
    }

    /// Blocks while the queue is full: this is backpressure.
    pub fn send(&self, v: T) {
        let mut q = self.q.lock().unwrap();
        while q.len() == self.cap {
            q = self.not_full.wait(q).unwrap(); // always re-check in a loop
        }
        q.push_back(v);
        self.not_empty.notify_one();
    }

    pub fn recv(&self) -> T {
        let mut q = self.q.lock().unwrap();
        loop {
            if let Some(v) = q.pop_front() {
                self.not_full.notify_one();
                return v;
            }
            q = self.not_empty.wait(q).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn zero_capacity_is_rejected() {
        let result = std::panic::catch_unwind(|| Bounded::<u8>::new(0));
        assert!(result.is_err());
    }
    #[test]
    fn many_producers_many_consumers() {
        let ch = Arc::new(Bounded::new(8));
        let producers: Vec<_> = (0..4)
            .map(|p| {
                let ch = ch.clone();
                std::thread::spawn(move || {
                    for i in 0..1000 {
                        ch.send(p * 1000 + i)
                    }
                })
            })
            .collect();
        let consumers: Vec<_> = (0..4)
            .map(|_| {
                let ch = ch.clone();
                std::thread::spawn(move || (0..1000).map(|_| ch.recv() as u64).sum::<u64>())
            })
            .collect();
        for p in producers {
            p.join().unwrap();
        }
        let total: u64 = consumers.into_iter().map(|c| c.join().unwrap()).sum();
        assert_eq!(total, (0..4000u64).sum());
    }
}
