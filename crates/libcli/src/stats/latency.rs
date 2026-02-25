use hdrhistogram::Histogram;

#[derive(Debug)]
#[allow(dead_code)]
pub struct LatencyStats {
    pub total: Histogram<u64>, // latency in ms
    pub per_sec: Histogram<u64>, // current second
}

impl Default for LatencyStats {
    fn default() -> Self {
        Self {
            total: Histogram::new_with_bounds(1, 120_000, 3).expect("LatencyStat Histogram create failed"),
            per_sec: Histogram::new_with_bounds(1, 120_000, 3).expect("LatencyStat Histogram create failed"),
        }
    }
}