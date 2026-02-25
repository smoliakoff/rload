#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct SecondSample {
    pub sec_index: u64,        // sec from start (0..)
    pub req_ok: u32,
    pub req_err: u32,
    pub latency_sum_ms: u64,
    pub latency_count: u32,

    pub p95_ms: u32,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct SlidingWindow {
    pub capacity: usize,              //  60 sec
    pub samples: std::collections::VecDeque<SecondSample>,
    pub current_sec: u64,
}