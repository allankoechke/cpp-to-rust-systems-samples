use std::time::Instant;
use tokio::{sync::mpsc, time::{sleep, Duration}};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<u64>(8); // bounded: send().await waits while the queue is full
    let start = Instant::now();
    let producer = tokio::spawn(async move {
        for i in 0..50u64 {
            tx.send(i).await.unwrap(); // pushes back on the producer instead of buffering without limit
            if i % 10 == 0 {
                println!("sent {i} at {:?}", start.elapsed());
            }
        }
    });
    let consumer = tokio::spawn(async move {
        while let Some(i) = rx.recv().await {
            sleep(Duration::from_millis(5)).await; // slow consumer
            if i % 10 == 0 {
                println!("consumed {i} at {:?}", start.elapsed());
            }
        }
    });
    producer.await.unwrap();
    consumer.await.unwrap();
    println!("total {:?}", start.elapsed());
}
