use tokio::time::Instant;
use crate::stats::latency::LatencyStats;
use crate::stats::window::SlidingWindow;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Totals {
    pub ticks_executed: u64,
    pub requests_ok: u64,
    pub requests_err: u64,
    pub in_flight: i32,

    pub _started_at: Instant,
    pub planned_total_ticks: u64,
    pub planned_duration_ms: u64,
}

impl Default for Totals {
    fn default() -> Self {
        Self {
            ticks_executed: 0,
            requests_ok: 0,
            requests_err: 0,
            in_flight: 0,
            _started_at: Instant::now(),

            planned_total_ticks: 0,
            planned_duration_ms: 0,
        }
    }
}


#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct LiveStats {
    pub totals: Totals,
    pub window: SlidingWindow,
    pub latency: LatencyStats,
    pub last_stage_index: u32,
}

#[allow(dead_code)]
struct LiveSnapshot {
    executed: u64,
    total: u64,
    rps_1s: u32,
    ok: u64,
    err: u64,
    err_rate: f64,
    avg_ms: u32,
    p95_ms: u32,
    in_flight: i32,
    eta_ms: u64,
}

impl From<LiveStats> for LiveSnapshot {
    fn from(value: LiveStats) -> Self {
        LiveSnapshot {
            executed: value.totals.ticks_executed,
            total: value.totals.planned_total_ticks,
            rps_1s: 0,
            ok: value.totals.requests_ok,
            err: value.totals.requests_err,
            err_rate: value.totals.requests_err.checked_div(value.totals.planned_total_ticks).unwrap_or(0) as f64,
            avg_ms: 0,
            p95_ms: 0,
            in_flight: 0,
            eta_ms: 0,
        }
    }
}