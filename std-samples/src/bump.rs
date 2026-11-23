use std::alloc::{alloc, dealloc, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// Minimal bump arena. It never runs destructors of what it hands out,
/// so use it for plain data (or leak-tolerant values) only.
pub struct Bump {
    start: NonNull<u8>,
    cap: usize,
    off: Cell<usize>,
}

impl Bump {
    pub fn with_capacity(cap: usize) -> Self {
        assert!(cap > 0);
        let layout = Layout::from_size_align(cap, 16).unwrap();
        // SAFETY: layout has non-zero size.
        let p = unsafe { alloc(layout) };
        Bump { start: NonNull::new(p).expect("out of memory"), cap, off: Cell::new(0) }
    }

    pub fn alloc<T>(&self, val: T) -> Option<&mut T> {
        let layout = Layout::new::<T>();
        let base = self.start.as_ptr() as usize;
        let cur = base + self.off.get();
        let aligned = (cur + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        if end > base + self.cap {
            return None;
        }
        self.off.set(end - base);
        let p = aligned as *mut T;
        // SAFETY: [aligned, end) lies inside our buffer, is aligned for T, and is
        // handed out exactly once because `off` only moves forward.
        unsafe {
            p.write(val);
            Some(&mut *p)
        }
    }
}

impl Drop for Bump {
    fn drop(&mut self) {
        // SAFETY: same layout as in `with_capacity`.
        unsafe { dealloc(self.start.as_ptr(), Layout::from_size_align(self.cap, 16).unwrap()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allocates_and_exhausts() {
        let b = Bump::with_capacity(64);
        let a = b.alloc(1u64).unwrap();
        let c = b.alloc(2u8).unwrap();
        let d = b.alloc(3u64).unwrap();
        assert_eq!((*a, *c, *d), (1, 2, 3));
        assert_eq!(d as *mut u64 as usize % 8, 0);
        assert!(b.alloc([0u8; 100]).is_none());
    }
}
