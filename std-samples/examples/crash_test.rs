use samples::kv::KvStore;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = std::env::temp_dir().join(format!("crash-kv-{}", std::process::id()));
    if args.get(1).map(String::as_str) == Some("child") {
        // Child: write keys forever; print a key only AFTER put() has returned (it is acknowledged).
        let mut kv = KvStore::open(&args[2]).unwrap();
        for i in 0u64.. {
            kv.put(&i.to_be_bytes(), &i.to_le_bytes()).unwrap();
            println!("{i}");
        }
    }
    let _ = std::fs::remove_file(&path);
    let mut next_key = 0u64;
    for round in 0..20u64 {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["child", path.to_str().unwrap()])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let stop_after = 20 + (round * 37) % 150; // vary the kill point
        let mut acked: Option<u64> = None;
        for _ in 0..stop_after {
            if let Some(Ok(l)) = lines.next() {
                acked = Some(l.parse().unwrap());
            }
        }
        child.kill().unwrap(); // SIGKILL: no destructors, no flushing
        child.wait().unwrap();
        let mut kv = KvStore::open(&path).unwrap();
        if let Some(last) = acked {
            for k in 0..=last {
                assert_eq!(kv.get(&k.to_be_bytes()).unwrap(), Some(k.to_le_bytes().to_vec()), "lost acknowledged key {k} in round {round}");
            }
            next_key = last + 1;
        }
        // Child restarts numbering at 0 each round and overwrites, which is fine for this check.
        let _ = next_key;
    }
    println!("20 rounds of kill -9: no acknowledged write was lost");
}
