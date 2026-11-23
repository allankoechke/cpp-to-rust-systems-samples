pub struct Bloom {
    bits: Vec<u64>,
    k: u32,
}

fn hash_pair(key: &[u8]) -> (u64, u64) {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a 64
    for &b in key {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (h, (h.rotate_left(31) ^ 0x9e37_79b9_7f4a_7c15) | 1) // double hashing: h1 + i*h2
}

impl Bloom {
    pub fn new(expected_keys: usize, bits_per_key: usize) -> Self {
        let nbits = (expected_keys * bits_per_key).max(64);
        let k = ((bits_per_key as f64 * 0.69) as u32).clamp(1, 16); // k = ln2 * bits/key
        Bloom { bits: vec![0; (nbits + 63) / 64], k }
    }
    fn positions(&self, key: &[u8]) -> impl Iterator<Item = (usize, u64)> + '_ {
        let (h1, h2) = hash_pair(key);
        let m = (self.bits.len() * 64) as u64;
        (0..self.k as u64).map(move |i| {
            let bit = h1.wrapping_add(i.wrapping_mul(h2)) % m;
            ((bit / 64) as usize, 1u64 << (bit % 64))
        })
    }
    pub fn insert(&mut self, key: &[u8]) {
        let pos: Vec<_> = self.positions(key).collect();
        for (w, mask) in pos { self.bits[w] |= mask; }
    }
    pub fn may_contain(&self, key: &[u8]) -> bool {
        self.positions(key).all(|(w, mask)| self.bits[w] & mask != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_false_negatives_and_few_false_positives() {
        let mut b = Bloom::new(10_000, 10);
        for i in 0..10_000u32 { b.insert(&i.to_le_bytes()); }
        assert!((0..10_000u32).all(|i| b.may_contain(&i.to_le_bytes())));
        let fp = (10_000..20_000u32).filter(|i| b.may_contain(&i.to_le_bytes())).count();
        assert!(fp < 300, "false positives: {fp}"); // expect about 1%
    }
}
