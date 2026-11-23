use samples::simd::{sum_avx2, sum_chunks, sum_scalar};
use std::hint::black_box;
use std::time::Instant;

#[allow(dead_code)]
struct ParticleAos { x: f32, y: f32, z: f32, mass: f32, name: [u8; 32] } // 48 bytes per particle

fn time<R>(label: &str, reps: u32, mut f: impl FnMut() -> R) {
    let t = Instant::now();
    for _ in 0..reps { black_box(f()); }
    println!("{label:<14} {:>8.2} ms/iter", t.elapsed().as_secs_f64() * 1e3 / reps as f64);
}

fn main() {
    let n = 4_000_000;
    let aos: Vec<ParticleAos> = (0..n).map(|i| ParticleAos { x: (i % 7) as f32, y: 0.0, z: 0.0, mass: 1.0, name: [0; 32] }).collect();
    let soa_x: Vec<f32> = aos.iter().map(|p| p.x).collect();
    time("AoS sum x", 20, || aos.iter().map(|p| p.x).sum::<f32>());
    time("SoA sum x", 20, || sum_scalar(black_box(&soa_x)));
    time("SoA chunks", 20, || sum_chunks(black_box(&soa_x)));
    time("SoA avx2", 20, || sum_avx2(black_box(&soa_x)));
}
