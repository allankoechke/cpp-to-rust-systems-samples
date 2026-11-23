use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use tokio::{io::{copy_bidirectional, AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Round-robin over the backends. A real proxy would skip unhealthy ones.
fn pick(backends: &[String]) -> &str {
    &backends[NEXT.fetch_add(1, Relaxed) % backends.len()]
}

async fn run_proxy(listener: TcpListener, backends: Vec<String>) -> std::io::Result<()> {
    loop {
        let (mut inbound, _) = listener.accept().await?;
        let target = pick(&backends).to_string();
        tokio::spawn(async move {
            match TcpStream::connect(&target).await {
                Ok(mut outbound) => {
                    // Shuttles bytes both ways until either side closes.
                    let _ = copy_bidirectional(&mut inbound, &mut outbound).await;
                }
                Err(e) => eprintln!("backend {target} unreachable: {e}"),
            }
        });
    }
}

// --- test scaffolding: two echo backends that tag their replies ---
async fn tagged_backend(tag: &'static str) -> String {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        loop {
            let (mut s, _) = l.accept().await.unwrap();
            tokio::spawn(async move {
                let mut b = [0u8; 64];
                let n = s.read(&mut b).await.unwrap();
                s.write_all(format!("{tag}:{}", String::from_utf8_lossy(&b[..n])).as_bytes()).await.unwrap();
            });
        }
    });
    addr
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let backends = vec![tagged_backend("A").await, tagged_backend("B").await];
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let proxy_addr = listener.local_addr()?;
    tokio::spawn(run_proxy(listener, backends));
    for _ in 0..4 {
        let mut c = TcpStream::connect(proxy_addr).await?;
        c.write_all(b"hi").await?;
        let mut reply = String::new();
        c.read_to_string(&mut reply).await?;
        println!("{reply}");
    }
    Ok(())
}
