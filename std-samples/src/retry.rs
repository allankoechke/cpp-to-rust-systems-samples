use std::time::Duration;

/// "Full jitter" exponential backoff: wait a random time between 0 and min(cap, base * 2^attempt).
/// `rand01` returns a number in [0, 1); inject it so tests are deterministic.
pub fn backoff(attempt: u32, base: Duration, cap: Duration, rand01: f64) -> Duration {
    let exp = base.saturating_mul(1u32.checked_shl(attempt).unwrap_or(u32::MAX));
    let ceiling = exp.min(cap);
    ceiling.mul_f64(rand01)
}

/// Retry `op` up to `max_attempts` times, sleeping with jittered backoff between failures.
pub fn retry<T, E>(
    max_attempts: u32,
    mut op: impl FnMut(u32) -> Result<T, E>,
    mut sleep: impl FnMut(Duration),
    mut rand01: impl FnMut() -> f64,
) -> Result<T, E> {
    let mut attempt = 0;
    loop {
        match op(attempt) {
            Ok(v) => return Ok(v),
            Err(e) if attempt + 1 >= max_attempts => return Err(e),
            Err(_) => {
                sleep(backoff(attempt, Duration::from_millis(50), Duration::from_secs(5), rand01()));
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backoff_grows_then_caps() {
        let (b, c) = (Duration::from_millis(100), Duration::from_secs(1));
        assert_eq!(backoff(0, b, c, 1.0), Duration::from_millis(100));
        assert_eq!(backoff(2, b, c, 1.0), Duration::from_millis(400));
        assert_eq!(backoff(10, b, c, 1.0), Duration::from_secs(1)); // capped
        assert_eq!(backoff(3, b, c, 0.0), Duration::ZERO);
    }
    #[test]
    fn retries_until_success() {
        let mut sleeps = 0;
        let r: Result<u32, &str> = retry(5, |n| if n < 2 { Err("flaky") } else { Ok(n) }, |_| sleeps += 1, || 0.5);
        assert_eq!(r, Ok(2));
        assert_eq!(sleeps, 2);
        let r: Result<u32, &str> = retry(3, |_| Err("down"), |_| {}, || 0.5);
        assert_eq!(r, Err("down"));
    }
}
