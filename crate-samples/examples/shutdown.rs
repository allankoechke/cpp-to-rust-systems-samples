use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
use tokio_util::task::TaskTracker;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let tracker = TaskTracker::new();
    let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(false);

    let conns = tracker.clone();
    let server = tokio::spawn(async move {
        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (mut sock, _) = res.unwrap();
                    conns.spawn(async move {              // tracked: shutdown will wait for it
                        let mut buf = [0u8; 4096];
                        loop {
                            match sock.read(&mut buf).await {
                                Ok(0) | Err(_) => return,
                                Ok(n) => { if sock.write_all(&buf[..n]).await.is_err() { return; } }
                            }
                        }
                    });
                }
                _ = stop_rx.changed() => break,           // stop accepting new connections
            }
        }
    });

    // ... run until a signal arrives, then:
    stop_tx.send(true).unwrap();
    server.await.unwrap();
    tracker.close();
    tracker.wait().await;                                  // in-flight work finishes (add a deadline)
    println!("clean shutdown");
    Ok(())
}
