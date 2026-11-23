use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::fs::FileExt;
use std::path::Path;

pub const PAGE_SIZE: usize = 4096;
pub type PageId = u64;

pub struct Pager { file: File, pages: u64 }

impl Pager {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).create(true).open(path)?;
        let len = file.metadata()?.len();
        if len % PAGE_SIZE as u64 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "file length is not a multiple of the page size"));
        }
        Ok(Pager { file, pages: len / PAGE_SIZE as u64 })
    }

    pub fn read(&self, id: PageId) -> io::Result<Box<[u8; PAGE_SIZE]>> {
        if id >= self.pages {
            return Err(io::Error::new(io::ErrorKind::NotFound, "page out of range"));
        }
        let mut buf = Box::new([0u8; PAGE_SIZE]);
        self.file.read_exact_at(&mut buf[..], id * PAGE_SIZE as u64)?;
        Ok(buf)
    }

    pub fn write(&self, id: PageId, page: &[u8; PAGE_SIZE]) -> io::Result<()> {
        self.file.write_all_at(page, id * PAGE_SIZE as u64)
    }

    pub fn allocate(&mut self) -> io::Result<PageId> {
        let id = self.pages;
        self.file.write_all_at(&[0u8; PAGE_SIZE], id * PAGE_SIZE as u64)?;
        self.pages += 1;
        Ok(id)
    }

    pub fn sync(&self) -> io::Result<()> { self.file.sync_data() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allocate_write_read() {
        let p = std::env::temp_dir().join(format!("pager-test-{}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        let mut pager = Pager::open(&p).unwrap();
        let id = pager.allocate().unwrap();
        let mut page = [0u8; PAGE_SIZE];
        page[..5].copy_from_slice(b"hello");
        pager.write(id, &page).unwrap();
        pager.sync().unwrap();
        assert_eq!(&pager.read(id).unwrap()[..5], b"hello");
        assert!(pager.read(99).is_err());
    }
}
