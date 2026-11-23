use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

/// Consistent-hash ring with virtual nodes.
/// DefaultHasher is not guaranteed stable across Rust versions; use a fixed hash
/// (xxhash, FNV) for anything that must agree between processes.
pub struct Ring { ring: BTreeMap<u64, String>, vnodes: usize }

fn h<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl Ring {
    pub fn new(vnodes: usize) -> Self { Ring { ring: BTreeMap::new(), vnodes } }
    pub fn add_node(&mut self, name: &str) {
        for i in 0..self.vnodes { self.ring.insert(h(&(name, i)), name.to_string()); }
    }
    pub fn remove_node(&mut self, name: &str) {
        for i in 0..self.vnodes { self.ring.remove(&h(&(name, i))); }
    }
    pub fn node_for(&self, key: &str) -> Option<&str> {
        let k = h(key);
        self.ring.range(k..).next().or_else(|| self.ring.iter().next()).map(|(_, n)| n.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adding_a_node_moves_only_some_keys() {
        let mut r = Ring::new(100);
        for n in ["a", "b", "c"] { r.add_node(n); }
        let keys: Vec<String> = (0..10_000).map(|i| format!("key{i}")).collect();
        let before: Vec<_> = keys.iter().map(|k| r.node_for(k).unwrap().to_string()).collect();
        r.add_node("d");
        let moved = keys.iter().zip(&before).filter(|(k, b)| r.node_for(k).unwrap() != b.as_str()).count();
        assert!(moved > 1_500 && moved < 3_500, "moved {moved}"); // roughly 1/4 should move
    }
}
