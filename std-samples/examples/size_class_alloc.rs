use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::UnsafeCell;
use std::ptr::null_mut;

const CLASSES: [usize; 8] = [16, 32, 48, 64, 128, 256, 384, 512];
const CHUNK: usize = 64 * 1024;

struct Local {
    free: [*mut u8; 8], // intrusive singly-linked free list per size class
    bump: *mut u8,
    left: usize,
}

thread_local! {
    // const-initialised and without Drop, so touching it never allocates or registers a destructor.
    static LOCAL: UnsafeCell<Local> =
        const { UnsafeCell::new(Local { free: [null_mut(); 8], bump: null_mut(), left: 0 }) };
}

fn class_of(size: usize) -> usize {
    CLASSES.iter().position(|&c| size <= c).unwrap()
}

pub struct SizeClass;

unsafe impl GlobalAlloc for SizeClass {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if l.size() > 512 || l.align() > 16 {
            // SAFETY: `l` is the layout from the allocator caller.
            return System.alloc(l);
        }
        let c = class_of(l.size());
        LOCAL.with(|cell| {
            // SAFETY: LOCAL is an UnsafeCell whose contents are unique to this thread.
            let loc = &mut *cell.get();
            let head = loc.free[c];
            if !head.is_null() {
                // SAFETY: a non-null free-list head was returned by this allocator
                // and still points to storage writable for one pointer-sized value.
                loc.free[c] = *(head as *mut *mut u8); // pop
                return head;
            }
            let sz = CLASSES[c];
            if loc.left < sz {
                // SAFETY: CHUNK and 16 form a valid non-zero Layout.
                let chunk = System.alloc(Layout::from_size_align_unchecked(CHUNK, 16));
                if chunk.is_null() {
                    return chunk;
                }
                loc.bump = chunk;
                loc.left = CHUNK; // the old chunk's tail is simply abandoned
            }
            let p = loc.bump;
            // SAFETY: loc.left >= sz and bump points to sz bytes within the current chunk.
            loc.bump = loc.bump.add(sz);
            loc.left -= sz;
            p
        })
    }

    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if l.size() > 512 || l.align() > 16 {
            // SAFETY: `p` and `l` identify memory allocated by System for this block.
            return System.dealloc(p, l);
        }
        let c = class_of(l.size());
        LOCAL.with(|cell| {
            // SAFETY: LOCAL is an UnsafeCell whose contents are unique to this thread.
            let loc = &mut *cell.get();
            // SAFETY: small blocks contain at least one pointer-sized storage slot
            // and are no longer in use by their previous owner.
            *(p as *mut *mut u8) = loc.free[c]; // push; a block freed on another thread joins that thread's list
            loc.free[c] = p;
        })
    }
}

#[global_allocator]
static A: SizeClass = SizeClass;

fn main() {
    let handles: Vec<_> = (0..4)
        .map(|t| {
            std::thread::spawn(move || {
                let mut keep = Vec::new();
                for i in 0..200_000usize {
                    let s = format!("thread {t} item {i}");
                    if i % 10 == 0 {
                        keep.push(s);
                    }
                    let b = Box::new([i; 7]);
                    assert_eq!(b[3], i);
                }
                keep.len()
            })
        })
        .collect();
    let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
    println!("ok, kept {total} strings");
}
