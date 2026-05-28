// Metrics collection for performance tests.
//
// Measures throughput (ops/sec) and latency percentiles (p50/p95/p99)
// across benchmark runs.

use std::time::Instant;

/// Result of a throughput benchmark.
#[derive(Debug)]
pub struct ThroughputResult {
    /// Total number of operations performed
    pub total_ops: u64,
    /// Elapsed wall clock time
    pub elapsed_ms: f64,
    /// Operations per second
    pub ops_per_sec: f64,
}

impl ThroughputResult {
    /// Calculate throughput metrics from total operations and elapsed time.
    #[must_use]
    pub fn new(total_ops: u64, elapsed_ms: f64) -> Self {
        let ops_per_sec = if elapsed_ms > 0.0 {
            (total_ops as f64 / elapsed_ms) * 1000.0
        } else {
            f64::INFINITY
        };
        Self {
            total_ops,
            elapsed_ms,
            ops_per_sec,
        }
    }

    /// Format as a human-readable string for console output.
    #[must_use]
    pub fn to_string(&self) -> String {
        format!(
            "{} ops in {:.2}ms ({:.0} ops/sec)",
            self.total_ops, self.elapsed_ms, self.ops_per_sec
        )
    }
}

/// Latency profile measured across multiple operations.
#[derive(Debug)]
pub struct LatencyProfile {
    /// Number of measurements
    pub count: usize,
    /// Mean latency in milliseconds
    pub mean_ms: f64,
    /// 50th percentile (median) in milliseconds
    pub p50_ms: f64,
    /// 95th percentile in milliseconds
    pub p95_ms: f64,
    /// 99th percentile in milliseconds
    pub p99_ms: f64,
    /// Minimum latency in milliseconds
    pub min_ms: f64,
    /// Maximum latency in milliseconds
    pub max_ms: f64,
}

impl LatencyProfile {
    /// Calculate a latency profile from a list of durations in microseconds.
    #[must_use]
    pub fn from_micros(durations: &[u64]) -> Self {
        if durations.is_empty() {
            return Self {
                count: 0,
                mean_ms: 0.0,
                p50_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                min_ms: 0.0,
                max_ms: 0.0,
            };
        }

        let mut sorted = durations.to_vec();
        sorted.sort_unstable();

        let sum: u64 = sorted.iter().sum();
        let mean = (sum as f64 / sorted.len() as f64) / 1000.0; // convert to ms
        let min = sorted[0] as f64 / 1000.0;
        let max = sorted[sorted.len() - 1] as f64 / 1000.0;

        Self {
            count: sorted.len(),
            mean_ms: mean,
            p50_ms: percentile(&sorted, 50) / 1000.0,
            p95_ms: percentile(&sorted, 95) / 1000.0,
            p99_ms: percentile(&sorted, 99) / 1000.0,
            min_ms: min,
            max_ms: max,
        }
    }

    /// Format as a human-readable string for console output.
    #[must_use]
    pub fn to_string(&self) -> String {
        format!(
            "n={} mean={:.2}ms p50={:.2}ms p95={:.2}ms p99={:.2}ms min={:.2}ms max={:.2}ms",
            self.count, self.mean_ms, self.p50_ms, self.p95_ms, self.p99_ms, self.min_ms, self.max_ms
        )
    }
}

/// Calculate the given percentile from a sorted slice.
fn percentile(sorted: &[u64], pct: u64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = (pct as f64 / 100.0 * sorted.len() as f64).floor() as usize;
    let index = index.min(sorted.len() - 1);
    sorted[index]
}

/// Benchmark a closure, returning throughput result and latencies.
///
/// # Arguments
/// * `iterations` — Number of times to call the closure
/// * `f` — The operation to benchmark
///
/// # Returns
/// `(ThroughputResult, LatencyProfile)`
pub fn benchmark<F>(iterations: u64, mut f: F) -> (ThroughputResult, LatencyProfile)
where
    F: FnMut(),
{
    let start = Instant::now();
    let mut latencies = Vec::with_capacity(iterations as usize);

    for _ in 0..iterations {
        let op_start = Instant::now();
        f();
        latencies.push(op_start.elapsed().as_micros() as u64);
    }

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

    (
        ThroughputResult::new(iterations, elapsed_ms),
        LatencyProfile::from_micros(&latencies),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_throughput_result_format() {
        let result = ThroughputResult::new(100, 50.0);
        assert_eq!(result.total_ops, 100);
        assert!((result.ops_per_sec - 2000.0).abs() < 1.0);
        let s = result.to_string();
        assert!(s.contains("100 ops"));
        assert!(s.contains("ops/sec"));
    }

    #[test]
    fn test_latency_profile() {
        let durations: Vec<u64> = (1..=100).collect(); // 1..=100 microseconds
        let profile = LatencyProfile::from_micros(&durations);
        assert_eq!(profile.count, 100);
        assert!(profile.p50_ms > 0.0);
        assert!(profile.p95_ms > profile.p50_ms);
        assert!(profile.min_ms > 0.0);
        assert!(profile.max_ms > 0.0);
    }

    #[test]
    fn test_latency_profile_empty() {
        let profile = LatencyProfile::from_micros(&[]);
        assert_eq!(profile.count, 0);
        assert_eq!(profile.mean_ms, 0.0);
    }

    #[test]
    fn test_benchmark_runs() {
        let mut counter = 0u64;
        let (result, _profile) = benchmark(100, || {
            counter += 1;
        });
        assert_eq!(counter, 100);
        assert_eq!(result.total_ops, 100);
        assert!(result.elapsed_ms > 0.0);
    }

    #[test]
    fn test_percentile_sorted() {
        let sorted: Vec<u64> = (1..=100).collect();
        assert_eq!(percentile(&sorted, 50), 50);
        assert_eq!(percentile(&sorted, 95), 95);
        assert_eq!(percentile(&sorted, 99), 99);
    }
}
