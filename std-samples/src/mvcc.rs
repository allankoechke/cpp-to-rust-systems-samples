use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Sort by user key ascending, then by timestamp DESCENDING, so the newest version comes first.
/// Compare as a tuple, never as concatenated bytes: a key that is a prefix of another would interleave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionedKey {
    pub user: Vec<u8>,
    pub ts: u64,
}

impl Ord for VersionedKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.user.cmp(&other.user).then_with(|| other.ts.cmp(&self.ts))
    }
}
impl PartialOrd for VersionedKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

#[derive(Default)]
pub struct MvccMap {
    map: BTreeMap<VersionedKey, Option<Vec<u8>>>, // None = tombstone at that timestamp
}

impl MvccMap {
    pub fn put(&mut self, key: &[u8], value: Option<Vec<u8>>, commit_ts: u64) {
        self.map.insert(VersionedKey { user: key.to_vec(), ts: commit_ts }, value);
    }

    /// The newest version of `key` with ts <= read_ts, or None if absent or deleted at that time.
    pub fn get(&self, key: &[u8], read_ts: u64) -> Option<&Vec<u8>> {
        let probe = VersionedKey { user: key.to_vec(), ts: read_ts };
        let (k, v) = self.map.range(probe..).next()?; // first entry at or after (key, read_ts)
        if k.user == key { v.as_ref() } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_reads() {
        let mut m = MvccMap::default();
        m.put(b"a", Some(b"v1".to_vec()), 10);
        m.put(b"a", Some(b"v2".to_vec()), 20);
        m.put(b"ab", Some(b"other".to_vec()), 15); // a key that has b"a" as a prefix
        m.put(b"a", None, 30); // delete at ts 30
        assert_eq!(m.get(b"a", 5), None); // before the first write
        assert_eq!(m.get(b"a", 10).map(|v| v.as_slice()), Some(&b"v1"[..]));
        assert_eq!(m.get(b"a", 25).map(|v| v.as_slice()), Some(&b"v2"[..]));
        assert_eq!(m.get(b"a", 99), None); // deleted
        assert_eq!(m.get(b"ab", 99).map(|v| v.as_slice()), Some(&b"other"[..]));
    }
}
