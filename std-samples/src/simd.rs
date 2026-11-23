pub fn sum_scalar(xs: &[f32]) -> f32 {
    xs.iter().sum() // strict left-to-right float adds: the compiler cannot vectorise this
}

/// Eight independent accumulators let LLVM vectorise without changing semantics.
pub fn sum_chunks(xs: &[f32]) -> f32 {
    let mut acc = [0f32; 8];
    let mut it = xs.chunks_exact(8);
    for c in &mut it {
        for i in 0..8 { acc[i] += c[i]; }
    }
    acc.iter().sum::<f32>() + it.remainder().iter().sum::<f32>()
}

#[cfg(target_arch = "x86_64")]
pub fn sum_avx2(xs: &[f32]) -> f32 {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: we just checked the CPU supports AVX2.
        unsafe { sum_avx2_impl(xs) }
    } else {
        sum_chunks(xs)
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn sum_avx2_impl(xs: &[f32]) -> f32 {
    use std::arch::x86_64::*;
    let mut acc = _mm256_setzero_ps();
    let mut it = xs.chunks_exact(8);
    for c in &mut it {
        acc = _mm256_add_ps(acc, _mm256_loadu_ps(c.as_ptr()));
    }
    let mut lanes = [0f32; 8];
    _mm256_storeu_ps(lanes.as_mut_ptr(), acc);
    lanes.iter().sum::<f32>() + it.remainder().iter().sum::<f32>()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn variants_agree() {
        let xs: Vec<f32> = (0..1003).map(|i| (i % 7) as f32).collect();
        let want = sum_scalar(&xs);
        assert!((sum_chunks(&xs) - want).abs() < 1e-2);
        #[cfg(target_arch = "x86_64")]
        assert!((sum_avx2(&xs) - want).abs() < 1e-2);
    }
}
