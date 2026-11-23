use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

const TOMBSTONE: u32 = u32::MAX;
const HEADER: u64 = 12; // crc u32 | klen u32 | vlen u32

/// FNV-1a: tiny and dependency-free. Swap in crc32fast for real use.
fn checksum(data: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for &b in data {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

pub struct KvStore {
    writer: File,
    reader: File,
    index: HashMap<Vec<u8>, (u64, u32)>, // key -> (value offset, value length)
    end: u64,
}

impl KvStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let writer = OpenOptions::new().create(true).append(true).open(path)?;
        let mut index = HashMap::new();
        let mut r = BufReader::new(File::open(path)?);
        let mut pos = 0u64;
        loop {
            let mut hdr = [0u8; HEADER as usize];
            if r.read_exact(&mut hdr).is_err() { break; } // clean EOF or torn header
            let crc = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
            let klen = u32::from_le_bytes(hdr[4..8].try_into().unwrap());
            let vlen = u32::from_le_bytes(hdr[8..12].try_into().unwrap());
            let vbytes = if vlen == TOMBSTONE { 0 } else { vlen };
            if klen > (1 << 20) || vbytes > (1 << 28) { break; } // implausible: treat as corruption
            let mut body = vec![0u8; (klen + vbytes) as usize];
            if r.read_exact(&mut body).is_err() { break; } // torn body
            let mut check = hdr[4..12].to_vec();
            check.extend_from_slice(&body);
            if checksum(&check) != crc { break; } // corrupt tail
            let key = body[..klen as usize].to_vec();
            if vlen == TOMBSTONE {
                index.remove(&key);
            } else {
                index.insert(key, (pos + HEADER + klen as u64, vlen));
            }
            pos += HEADER + body.len() as u64;
        }
        // Drop any torn tail so new appends start at a record boundary.
        if writer.metadata()?.len() > pos {
            writer.set_len(pos)?;
        }
        Ok(KvStore { writer, reader: File::open(path)?, index, end: pos })
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) -> io::Result<()> {
        let off = self.append(key, Some(value))?;
        self.index.insert(key.to_vec(), (off, value.len() as u32));
        Ok(())
    }

    pub fn delete(&mut self, key: &[u8]) -> io::Result<()> {
        self.append(key, None)?;
        self.index.remove(key);
        Ok(())
    }

    pub fn get(&mut self, key: &[u8]) -> io::Result<Option<Vec<u8>>> {
        let Some(&(off, len)) = self.index.get(key) else { return Ok(None) };
        self.reader.seek(SeekFrom::Start(off))?;
        let mut buf = vec![0u8; len as usize];
        self.reader.read_exact(&mut buf)?;
        Ok(Some(buf))
    }

    fn append(&mut self, key: &[u8], value: Option<&[u8]>) -> io::Result<u64> {
        let vlen = value.map_or(TOMBSTONE, |v| v.len() as u32);
        let mut body = Vec::with_capacity(8 + key.len());
        body.extend_from_slice(&(key.len() as u32).to_le_bytes());
        body.extend_from_slice(&vlen.to_le_bytes());
        body.extend_from_slice(key);
        if let Some(v) = value { body.extend_from_slice(v); }
        let mut rec = checksum(&body).to_le_bytes().to_vec();
        rec.extend_from_slice(&body);
        self.writer.write_all(&rec)?;
        self.writer.sync_data()?; // durability point: the write is acknowledged after this
        let value_off = self.end + HEADER + key.len() as u64;
        self.end += rec.len() as u64;
        Ok(value_off)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tmp(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("kv-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }
    #[test]
    fn put_get_delete_and_reopen() {
        let p = tmp("basic");
        {
            let mut kv = KvStore::open(&p).unwrap();
            kv.put(b"a", b"1").unwrap();
            kv.put(b"b", b"2").unwrap();
            kv.put(b"a", b"3").unwrap();
            kv.delete(b"b").unwrap();
        }
        let mut kv = KvStore::open(&p).unwrap();
        assert_eq!(kv.get(b"a").unwrap(), Some(b"3".to_vec()));
        assert_eq!(kv.get(b"b").unwrap(), None);
    }
    #[test]
    fn survives_torn_tail() {
        let p = tmp("torn");
        { let mut kv = KvStore::open(&p).unwrap(); kv.put(b"k", b"v").unwrap(); }
        // Simulate a crash mid-write: append half a record of junk.
        OpenOptions::new().append(true).open(&p).unwrap().write_all(&[1, 2, 3, 4, 5, 6, 7]).unwrap();
        let mut kv = KvStore::open(&p).unwrap();
        assert_eq!(kv.get(b"k").unwrap(), Some(b"v".to_vec()));
        kv.put(b"k2", b"v2").unwrap();
        drop(kv);
        let mut kv = KvStore::open(&p).unwrap();
        assert_eq!(kv.get(b"k2").unwrap(), Some(b"v2".to_vec()));
    }
}
