use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub type Entry = (Vec<u8>, Option<Vec<u8>>); // None = tombstone

/// Merge sorted runs into one. `runs[0]` is the NEWEST run: when the same key
/// appears in several runs, the newest version wins.
/// Drop tombstones only when merging into the bottom level, where nothing older exists.
pub fn merge(runs: Vec<Vec<Entry>>, drop_tombstones: bool) -> Vec<Entry> {
    let mut iters: Vec<_> = runs.into_iter().map(|r| r.into_iter()).collect();
    let mut heads: Vec<Option<Entry>> = Vec::new();
    let mut heap = BinaryHeap::new(); // min-heap on (key, run index): ties go to the newer run
    for (i, it) in iters.iter_mut().enumerate() {
        let e = it.next();
        if let Some((k, _)) = &e {
            heap.push(Reverse((k.clone(), i)));
        }
        heads.push(e);
    }
    let mut out: Vec<Entry> = Vec::new();
    while let Some(Reverse((key, i))) = heap.pop() {
        let entry = heads[i].take().unwrap();
        if let Some(next) = iters[i].next() {
            heap.push(Reverse((next.0.clone(), i)));
            heads[i] = Some(next);
        }
        if out.last().map_or(false, |(k, _)| *k == key) {
            continue; // an older version of a key we already emitted
        }
        out.push(entry);
    }
    if drop_tombstones {
        out.retain(|(_, v)| v.is_some());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn e(k: &str, v: Option<&str>) -> Entry { (k.as_bytes().to_vec(), v.map(|v| v.as_bytes().to_vec())) }
    #[test]
    fn newest_wins_and_tombstones_drop() {
        let newest = vec![e("b", None), e("c", Some("new"))];
        let oldest = vec![e("a", Some("1")), e("b", Some("2")), e("c", Some("old"))];
        let kept = merge(vec![newest.clone(), oldest.clone()], false);
        assert_eq!(kept, vec![e("a", Some("1")), e("b", None), e("c", Some("new"))]);
        let bottom = merge(vec![newest, oldest], true);
        assert_eq!(bottom, vec![e("a", Some("1")), e("c", Some("new"))]);
    }
}
