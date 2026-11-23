use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

pub fn prefix_sum(xs: &mut [u64]) {
    let chunk = (xs.len() / rayon::current_num_threads()).max(1024);
    // Pass 1: local prefix sums inside each chunk; collect each chunk's total.
    let totals: Vec<u64> = xs
        .par_chunks_mut(chunk)
        .map(|c| {
            let mut acc = 0;
            for x in c.iter_mut() { acc += *x; *x = acc; }
            acc
        })
        .collect();
    // Pass 2: exclusive scan of the (few) totals, sequentially.
    let mut offsets = Vec::with_capacity(totals.len());
    let mut run = 0;
    for t in &totals { offsets.push(run); run += t; }
    // Pass 3: add each chunk's offset.
    xs.par_chunks_mut(chunk).zip(offsets.par_iter()).for_each(|(c, &off)| {
        for x in c.iter_mut() { *x += off; }
    });
}

pub fn blur(src: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut dst = vec![0f32; w * h];
    dst.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, out) in row.iter_mut().enumerate() {
            let (mut sum, mut n) = (0f32, 0f32);
            for dy in -2i64..=2 {
                for dx in -2i64..=2 {
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

fn hammer<T: Sync>(slots: &[T], get: impl Fn(&T) -> &AtomicU64 + Sync) -> f64 {
    let t = Instant::now();
    std::thread::scope(|s| {
        for slot in slots {
            let get = &get;
            s.spawn(move || { for _ in 0..20_000_000 { get(slot).fetch_add(1, Relaxed); } });
        }
    });
    t.elapsed().as_secs_f64() * 1e3
}

fn main() {
    // prefix sum correctness
    let mut v: Vec<u64> = (1..=100_000).collect();
    let want: Vec<u64> = v.iter().scan(0, |a, &x| { *a += x; Some(*a) }).collect();
    prefix_sum(&mut v);
    assert_eq!(v, want);
    println!("prefix_sum ok");

    // speedup vs threads
    let (w, h) = (2048, 2048);
    let img: Vec<f32> = (0..w * h).map(|i| (i % 251) as f32).collect();
    for threads in [1, 2, 4] {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        let t = Instant::now();
        let out = pool.install(|| blur(&img, w, h));
        println!("blur {threads} thread(s): {:>7.1} ms (checksum {:.1})", t.elapsed().as_secs_f64() * 1e3, out[w * 5 + 5]);
    }

    // false sharing
    let adjacent: Vec<AtomicU64> = (0..4).map(|_| AtomicU64::new(0)).collect();
    let padded: Vec<Padded> = (0..4).map(|_| Padded(AtomicU64::new(0))).collect();
    println!("adjacent counters: {:>7.1} ms", hammer(&adjacent, |a| a));
    println!("padded counters:   {:>7.1} ms", hammer(&padded, |p| &p.0));
}
