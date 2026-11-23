use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// Metrics as plain atomics, rendered in the Prometheus text exposition format.
pub struct Metrics {
    pub requests_total: AtomicU64,
    pub errors_total: AtomicU64,
    pub in_flight: AtomicU64,
    // latency histogram buckets (upper bounds in ms), cumulative at render time
    buckets: [AtomicU64; 6],
    pub latency_sum_ms: AtomicU64,
}

const BOUNDS_MS: [u64; 5] = [1, 5, 10, 50, 100]; // the sixth bucket is +Inf

impl Metrics {
    pub const fn new() -> Self {
        const Z: AtomicU64 = AtomicU64::new(0);
        Metrics { requests_total: Z, errors_total: Z, in_flight: Z, buckets: [Z; 6], latency_sum_ms: Z }
    }

    pub fn observe_latency_ms(&self, ms: u64) {
        let i = BOUNDS_MS.iter().position(|&b| ms <= b).unwrap_or(5);
        self.buckets[i].fetch_add(1, Relaxed);
        self.latency_sum_ms.fetch_add(ms, Relaxed);
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        out += "# TYPE http_requests_total counter\n";
        out += &format!("http_requests_total {}\n", self.requests_total.load(Relaxed));
        out += "# TYPE http_errors_total counter\n";
        out += &format!("http_errors_total {}\n", self.errors_total.load(Relaxed));
        out += "# TYPE http_in_flight gauge\n";
        out += &format!("http_in_flight {}\n", self.in_flight.load(Relaxed));
        out += "# TYPE http_request_duration_ms histogram\n";
        let mut cumulative = 0;
        for (i, b) in BOUNDS_MS.iter().enumerate() {
            cumulative += self.buckets[i].load(Relaxed);
            out += &format!("http_request_duration_ms_bucket{{le=\"{b}\"}} {cumulative}\n");
        }
        cumulative += self.buckets[5].load(Relaxed);
        out += &format!("http_request_duration_ms_bucket{{le=\"+Inf\"}} {cumulative}\n");
        out += &format!("http_request_duration_ms_sum {}\n", self.latency_sum_ms.load(Relaxed));
        out += &format!("http_request_duration_ms_count {cumulative}\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_cumulative_buckets() {
        static M: Metrics = Metrics::new();
        M.requests_total.fetch_add(3, Relaxed);
        M.observe_latency_ms(1);
        M.observe_latency_ms(7);
        M.observe_latency_ms(500);
        let text = M.render();
        assert!(text.contains("http_requests_total 3"));
        assert!(text.contains("http_request_duration_ms_bucket{le=\"10\"} 2"));
        assert!(text.contains("http_request_duration_ms_bucket{le=\"+Inf\"} 3"));
    }
}
