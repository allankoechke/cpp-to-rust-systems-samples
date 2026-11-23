use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::task::TaskTracker;

pub fn blur(src: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut dst = vec![0f32; w * h];
    dst.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, out) in row.iter_mut().enumerate() {
            let (mut sum, mut n) = (0f32, 0f32);
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (yy, xx) = (y as i64 + dy, x as i64 + dx);
                    if yy >= 0 && yy < h as i64 && xx >= 0 && xx < w as i64 {
                        sum += src[yy as usize * w + xx as usize];
                        n += 1.0;
                    }
                }
            }
            *out = sum / n;
        }
    });
    dst
}

#[repr(align(64))]
struct Padded(AtomicU64);

static REQUESTS: AtomicU64 = AtomicU64::new(0);

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let img = vec![1f32; 64 * 64];
    let out = blur(&img, 64, 64);
    println!("blur ok {}", out[100]);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let s: u64 = pool.install(|| (0..1000u64).into_par_iter().sum());
    let padded = Padded(AtomicU64::new(s));
    println!("padded sum={}", padded.0.load(Relaxed));
    let f = std::fs::File::open("/etc/hostname")?;
    // SAFETY: the file descriptor remains open, and the mapping is not
    // concurrently truncated or modified while its bytes are being read.
    let map = unsafe { memmap2::Mmap::map(&f)? };
    println!("mmap bytes {}", map.len());

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let tracker = TaskTracker::new();
    let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(false);
    let addr = listener.local_addr()?;
    let t2 = tracker.clone();
    let server = tokio::spawn(async move {
        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (mut sock, _) = res.unwrap();
                    t2.spawn(async move {
                        let mut buf = [0u8; 4096];
                        loop {
                            match sock.read(&mut buf).await {
                                Ok(0) | Err(_) => return,
                                Ok(n) => { REQUESTS.fetch_add(1, Relaxed); if sock.write_all(&buf[..n]).await.is_err() { return; } }
                            }
                        }
                    });
                }
                _ = stop_rx.changed() => break,
            }
        }
    });
    let mut c = tokio::net::TcpStream::connect(addr).await?;
    c.write_all(b"ping").await?;
    let mut b = [0u8; 4];
    c.read_exact(&mut b).await?;
    println!("echo {:?}", std::str::from_utf8(&b));
    drop(c);
    stop_tx.send(true).unwrap();
    server.await.unwrap();
    tracker.close();
    tracker.wait().await;
    println!("drained, requests={}", REQUESTS.load(Relaxed));
    Ok(())
}
