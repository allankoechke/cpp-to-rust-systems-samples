use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

pub type MemTable = BTreeMap<Vec<u8>, Option<Vec<u8>>>; // None = tombstone

/// File layout:  [entries][index][footer]
///   entry:  klen u32 | vlen u32 (u32::MAX = tombstone) | key | value
///   index:  count u32, then per sampled entry: klen u32 | key | offset u64
///   footer: index_offset u64
pub fn flush(mem: &MemTable, path: &Path, sample_every: usize) -> io::Result<()> {
    let mut w = BufWriter::new(File::create(path)?);
    let mut offset = 0u64;
    let mut index: Vec<(&[u8], u64)> = Vec::new();
    for (i, (k, v)) in mem.iter().enumerate() {
        if i % sample_every == 0 { index.push((k.as_slice(), offset)); }
        let vlen = v.as_ref().map_or(u32::MAX, |v| v.len() as u32);
        w.write_all(&(k.len() as u32).to_le_bytes())?;
        w.write_all(&vlen.to_le_bytes())?;
        w.write_all(k)?;
        if let Some(v) = v { w.write_all(v)?; }
        offset += 8 + k.len() as u64 + v.as_ref().map_or(0, |v| v.len()) as u64;
    }
    let index_offset = offset;
    w.write_all(&(index.len() as u32).to_le_bytes())?;
    for (k, off) in &index {
        w.write_all(&(k.len() as u32).to_le_bytes())?;
        w.write_all(k)?;
        w.write_all(&off.to_le_bytes())?;
    }
    w.write_all(&index_offset.to_le_bytes())?;
    w.flush()?;
    w.get_ref().sync_all()
}

#[derive(Debug, PartialEq)]
pub enum Lookup { Found(Vec<u8>), Deleted, Missing }

pub struct SsTable { file: File, index: Vec<(Vec<u8>, u64)>, data_end: u64 }

impl SsTable {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let len = file.metadata()?.len();
        let (mut b4, mut b8) = ([0u8; 4], [0u8; 8]);
        file.seek(SeekFrom::Start(len - 8))?;
        file.read_exact(&mut b8)?;
        let index_offset = u64::from_le_bytes(b8);
        file.seek(SeekFrom::Start(index_offset))?;
        file.read_exact(&mut b4)?;
        let n = u32::from_le_bytes(b4);
        let mut index = Vec::with_capacity(n as usize);
        for _ in 0..n {
            file.read_exact(&mut b4)?;
            let mut k = vec![0u8; u32::from_le_bytes(b4) as usize];
            file.read_exact(&mut k)?;
            file.read_exact(&mut b8)?;
            index.push((k, u64::from_le_bytes(b8)));
        }
        Ok(SsTable { file, index, data_end: index_offset })
    }

    pub fn get(&mut self, key: &[u8]) -> io::Result<Lookup> {
        // Last sampled key <= target tells us which block to scan.
        let i = self.index.partition_point(|(k, _)| k.as_slice() <= key);
        if i == 0 { return Ok(Lookup::Missing); }
        let start = self.index[i - 1].1;
        let end = self.index.get(i).map_or(self.data_end, |e| e.1);
        let mut block = vec![0u8; (end - start) as usize];
        self.file.seek(SeekFrom::Start(start))?;
        self.file.read_exact(&mut block)?;
        let mut p = 0;
        while p < block.len() {
            let klen = u32::from_le_bytes(block[p..p + 4].try_into().unwrap()) as usize;
            let vlen = u32::from_le_bytes(block[p + 4..p + 8].try_into().unwrap());
            let k = &block[p + 8..p + 8 + klen];
            let vbytes = if vlen == u32::MAX { 0 } else { vlen as usize };
            if k == key {
                return Ok(if vlen == u32::MAX { Lookup::Deleted } else { Lookup::Found(block[p + 8 + klen..p + 8 + klen + vbytes].to_vec()) });
            }
            p += 8 + klen + vbytes;
        }
        Ok(Lookup::Missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flush_and_lookup() {
        let mut m = MemTable::new();
        for i in 0..100u32 { m.insert(format!("key{i:03}").into_bytes(), Some(format!("val{i}").into_bytes())); }
        m.insert(b"key050".to_vec(), None);
        let p = std::env::temp_dir().join(format!("sst-test-{}", std::process::id()));
        flush(&m, &p, 16).unwrap();
        let mut t = SsTable::open(&p).unwrap();
        assert_eq!(t.get(b"key042").unwrap(), Lookup::Found(b"val42".to_vec()));
        assert_eq!(t.get(b"key050").unwrap(), Lookup::Deleted);
        assert_eq!(t.get(b"zzz").unwrap(), Lookup::Missing);
        assert_eq!(t.get(b"aaa").unwrap(), Lookup::Missing);
    }
}
