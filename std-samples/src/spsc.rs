use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{
    AtomicUsize,
    Ordering::{Acquire, Relaxed, Release},
};
use std::sync::Arc;

struct Shared<T> {
    buf: Box<[UnsafeCell<MaybeUninit<T>>]>,
    head: AtomicUsize, // next slot to read; written only by the consumer
    tail: AtomicUsize, // next slot to write; written only by the producer
}

// SAFETY: the producer/consumer split below guarantees each slot is accessed by
// one thread at a time, with hand-off ordered by the Acquire/Release pairs.
unsafe impl<T: Send> Sync for Shared<T> {}

pub struct Producer<T>(Arc<Shared<T>>);
pub struct Consumer<T>(Arc<Shared<T>>);

pub fn channel<T>(cap: usize) -> (Producer<T>, Consumer<T>) {
    assert!(cap > 0);
    let buf = (0..cap)
        .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
        .collect();
    let s = Arc::new(Shared {
        buf,
        head: AtomicUsize::new(0),
        tail: AtomicUsize::new(0),
    });
    (Producer(s.clone()), Consumer(s))
}

impl<T> Producer<T> {
    pub fn push(&mut self, v: T) -> Result<(), T> {
        let s = &*self.0;
        let t = s.tail.load(Relaxed); // only we write tail
        let h = s.head.load(Acquire); // see the consumer's frees
        if t.wrapping_sub(h) == s.buf.len() {
            return Err(v); // full
        }
        // SAFETY: slot t is empty and owned by the producer until tail is published.
        unsafe {
            (*s.buf[t % s.buf.len()].get()).write(v);
        }
        s.tail.store(t.wrapping_add(1), Release); // publish the write
        Ok(())
    }
}

impl<T> Consumer<T> {
    pub fn pop(&mut self) -> Option<T> {
        let s = &*self.0;
        let h = s.head.load(Relaxed); // only we write head
        let t = s.tail.load(Acquire); // see the producer's writes
        if h == t {
            return None; // empty
        }
        // SAFETY: slot h was initialised before tail passed it, and is ours until head moves.
        let v = unsafe { (*s.buf[h % s.buf.len()].get()).assume_init_read() };
        s.head.store(h.wrapping_add(1), Release); // free the slot
        Some(v)
    }
}

impl<T> Drop for Shared<T> {
    fn drop(&mut self) {
        let (h, t) = (*self.head.get_mut(), *self.tail.get_mut());
        let occupied = t.wrapping_sub(h);
        for i in 0..occupied {
            let slot = h.wrapping_add(i) % self.buf.len();
            // SAFETY: the producer/consumer are both gone, and each of these slots
            // contains a value that has not yet been moved out by the consumer.
            unsafe {
                (*self.buf[slot].get()).assume_init_drop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transfers_in_order() {
        let (mut p, mut c) = channel::<u64>(64);
        let prod = std::thread::spawn(move || {
            for i in 0..200_000u64 {
                while p.push(i).is_err() {
                    std::hint::spin_loop();
                }
            }
        });
        let cons = std::thread::spawn(move || {
            let mut next = 0u64;
            while next < 200_000 {
                if let Some(v) = c.pop() {
                    assert_eq!(v, next);
                    next += 1;
                }
            }
        });
        prod.join().unwrap();
        cons.join().unwrap();
    }
    #[test]
    fn drops_leftovers() {
        let (mut p, _c) = channel::<String>(4);
        p.push("a".into()).unwrap();
        p.push("b".into()).unwrap();
    }
}
