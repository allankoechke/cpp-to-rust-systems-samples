use std::ffi::{c_char, CStr, CString};
use std::ptr::NonNull;

#[repr(C)]
pub struct RawCounter {
    _private: [u8; 0], // opaque: Rust never looks inside
}

extern "C" {
    fn counter_new(start: i64) -> *mut RawCounter;
    fn counter_incr(c: *mut RawCounter) -> i64;
    fn counter_free(c: *mut RawCounter);
    fn strlen(s: *const c_char) -> usize;
    fn getenv(name: *const c_char) -> *const c_char;
}

/// Safe wrapper: owns the C object and frees it exactly once.
pub struct Counter {
    raw: NonNull<RawCounter>,
}

impl Counter {
    pub fn new(start: i64) -> Option<Counter> {
        // SAFETY: counter_new has no preconditions; it returns null on failure.
        NonNull::new(unsafe { counter_new(start) }).map(|raw| Counter { raw })
    }
    pub fn incr(&mut self) -> i64 {
        // SAFETY: `raw` is valid until Drop, and &mut self guarantees exclusive access.
        unsafe { counter_incr(self.raw.as_ptr()) }
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        // SAFETY: `raw` came from counter_new and is freed only here.
        unsafe { counter_free(self.raw.as_ptr()) }
    }
}
// NonNull makes Counter !Send and !Sync by default. Only add `unsafe impl Send`
// after reading the C library's thread-safety contract.

pub fn c_strlen(s: &str) -> Option<usize> {
    let c = CString::new(s).ok()?; // fails if s contains an interior NUL
    // SAFETY: c is a valid NUL-terminated string that outlives the call.
    Some(unsafe { strlen(c.as_ptr()) })
}

pub fn env(name: &str) -> Option<String> {
    let n = CString::new(name).ok()?;
    // SAFETY: n is valid; getenv returns null or a valid C string we copy immediately.
    let p = unsafe { getenv(n.as_ptr()) };
    if p.is_null() { None } else { Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()) }
}

fn main() {
    let mut c = Counter::new(41).expect("alloc failed");
    println!("{}", c.incr());
    println!("{:?} {:?}", c_strlen("hello"), env("HOME"));
}
