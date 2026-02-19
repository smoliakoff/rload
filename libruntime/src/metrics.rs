use std::cmp::{max, min};
use std::collections::{BTreeMap, HashMap};
use serde::Serialize;
use crate::run_engine::{EndpointStats, LatencyMs};
use crate::vu_runner::{ ResponseResult};

#[derive(Debug, serde::Serialize)]
pub struct MetricsAggregator {
    pub total_requests: u64,
    pub ok_requests: u64,
    pub error_requests: u64,

    pub latency: Vec<u64>,
    pub latency_min: u64,
    pub latency_max:u64,
    pub latency_sum:u64,

    ///  map endpoint_key → count, ok, err, latency_sum
    pub by_endpoint: BTreeMap<String, EndpointStats>,
    /// map journey_name → count (сколько раз выбрали journey для выполнения реквеста)
    pub by_journey: BTreeMap<String, (usize, u64)>
}

impl MetricsAggregator {
    pub(crate) fn new() -> Self{
        Self {
            total_requests: 0,
            ok_requests: 0,
            error_requests: 0,
            latency_min: u64::MIN,
            latency_max: 0,
            latency_sum: 0,
            latency: vec!(0),
            by_endpoint: BTreeMap::new(),
            by_journey: BTreeMap::new(),
        }
    }
    pub fn consume(&mut self, request_event: ResponseResult) {
        self.total_requests += 1;
        if (request_event.ok) {
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

        self.by_endpoint.entry(request_event.endpoint_key).and_modify(|endpoint_metrics| {
            endpoint_metrics.request.total +=1;
            if request_event.ok {
                endpoint_metrics.request.ok +=1;
            } else {
                endpoint_metrics.request.error +=1;
            }
            endpoint_metrics.latency_ms = LatencyMs {
                min: min(endpoint_metrics.latency_ms.min, request_event.latency_ms),
                max: max(endpoint_metrics.latency_ms.max, request_event.latency_ms),
                avg: (endpoint_metrics.latency_ms.max + endpoint_metrics.latency_ms.min)/2,
            }
        }).or_insert(EndpointStats::default());
        
        self.by_journey.entry(request_event.journey_name).and_modify(|(journey_id, journey_count)| *journey_count += 1)
            .or_insert((request_event.journey_id as usize, 1));
    }
}