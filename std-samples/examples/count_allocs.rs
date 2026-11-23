use samples::parser::parse_all;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        // SAFETY: `l` satisfies the GlobalAlloc contract and is forwarded unchanged.
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: `p` and `l` come from this allocator's earlier successful alloc.
        System.dealloc(p, l)
    }
}

#[global_allocator]
static A: Counting = Counting;

fn main() {
    let input = "INFO 1 started\nWARN 2 disk almost full\nERROR 3 crashed\n".repeat(1000);
    let before = ALLOCS.load(Relaxed);
    let total_ts: u64 = parse_all(&input).map(|r| r.ts).sum();
    let after = ALLOCS.load(Relaxed);
    println!(
        "sum={total_ts} allocations during parse: {}",
        after - before
    );
    assert_eq!(after - before, 0);
}
