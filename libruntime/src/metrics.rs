use crate::run_engine::{ByStage, EndpointStats, HIGHEST_US, LOWEST_US, SIGFIG};
use crate::vu_runner::ResponseResult;
use std::collections::BTreeMap;
use hdrhistogram::Histogram;

#[derive(Debug)]
pub struct MetricsAggregator {
    pub total_requests: u64,
    pub ok_requests: u64,
    pub error_requests: u64,

    pub overall_latency: Histogram<u64>,
    pub latency_by_stage: BTreeMap<u64, Histogram<u64>>,
    pub latency_by_endpoint: BTreeMap<String, Histogram<u64>>,
    pub latency: Vec<u64>,
    pub latency_min: u64,
    pub latency_max:u64,
    pub latency_sum: u64,

    ///  map endpoint_key → count, ok, err, latency_sum
    pub by_endpoint: BTreeMap<String, EndpointStats>,
    /// map journey_name → count (сколько раз выбрали journey для выполнения реквеста)
    pub by_journey: BTreeMap<String, (usize, u64)>,

    pub by_stage: BTreeMap<u64, ByStage>
}

impl MetricsAggregator {
    pub fn new() -> Self{
        Self {
            total_requests: 0,
            ok_requests: 0,
            error_requests: 0,
            latency_min: u64::MAX,
            latency_max: 0,
            latency_sum: 0,
            latency: vec!(0),
            by_endpoint: BTreeMap::new(),
            by_stage: BTreeMap::new(),
            by_journey: BTreeMap::new(),
            overall_latency: Histogram::new_with_bounds(LOWEST_US, HIGHEST_US, SIGFIG).expect("histogram over_all creation failed"),
            latency_by_stage: BTreeMap::new(),
            latency_by_endpoint: Default::default(),
        }
    }

    pub fn record_overall_latency(&mut self, latency_us: u64) {
        self.overall_latency.record(latency_us).expect("failed to record latency");
    }

    pub fn consume(&mut self, request_event: ResponseResult, now_ms: u64) {
        self.total_requests += 1;

        if request_event.ok {
            self.ok_requests += 1;
        }
        if !request_event.ok {
            self.error_requests += 1;
        }
        if request_event.latency_ms < self.latency_min {
            self.latency_min = request_event.latency_ms;
        }
        if request_event.latency_ms > self.latency_max {
            self.latency_max = request_event.latency_ms;
        }
        self.latency_sum += request_event.latency_ms;

        self.latency.push(request_event.latency_ms);

        self.record_overall_latency(request_event.latency_us);

        self.by_endpoint.entry(request_event.endpoint_key.clone()).and_modify(|endpoint_metrics| {
            endpoint_metrics.request.total +=1;
            if endpoint_metrics.request.total == 1 {
                endpoint_metrics.first_at_ms = now_ms;
            }
            endpoint_metrics.last_at_ms = now_ms;
            let window_ms = endpoint_metrics
                .last_at_ms
                .saturating_sub(endpoint_metrics.first_at_ms)
                .max(1);
            endpoint_metrics.achieved_rps =
                (endpoint_metrics.request.total as f64 / (window_ms as f64 / 1000.0)).round();

            endpoint_metrics.count += 1;
            if request_event.ok {
                endpoint_metrics.request.ok +=1;
            } else {
                endpoint_metrics.request.error +=1;
            }
        }).or_insert(EndpointStats::default());

        self.latency_by_endpoint.entry(request_event.endpoint_key).and_modify(|hist| {
            let _ = hist.record(request_event.latency_us).expect("cant crate record in hist for by endpoint");
        }).or_insert({
            let mut hist = Histogram::new_with_bounds(LOWEST_US, HIGHEST_US, SIGFIG).expect("histogram by endpoint creation failed");
            let _ = hist.record(request_event.latency_us);
            hist
        });

        self.by_stage.entry(request_event.stage_index).and_modify(|stage_rps| {
            stage_rps.request_count += 1;
            stage_rps.stage_duration_ms = now_ms.saturating_sub(stage_rps.stage_started_ms);
            let secs = (stage_rps.stage_duration_ms as f64 / 1000.0).max(0.001);
            stage_rps.achieved_rps = (stage_rps.request_count as f64 / secs) as u64;
        }).or_insert(
            ByStage {
                stage_index: request_event.stage_index,
                achieved_rps: 0,
                request_count: 1,
                stage_started_ms: request_event.stage_start_ms,
                stage_duration_ms: 0,
            }
        );
        self.latency_by_stage.entry(request_event.stage_index).and_modify(|hist| {
            hist.record(request_event.latency_us).expect("cant crate record in hist for by stage");
        }).or_insert({
            let mut hist = Histogram::new_with_bounds(LOWEST_US, HIGHEST_US, SIGFIG).expect("histogram by stage creation failed");
            hist.record(request_event.latency_us).expect("cant crate record in hist for by endpoint");
            hist
        });
        
        self.by_journey.entry(request_event.journey_name).and_modify(|(_journey_id, journey_count)| *journey_count += 1)
            .or_insert((request_event.journey_id as usize, 1));
    }
}