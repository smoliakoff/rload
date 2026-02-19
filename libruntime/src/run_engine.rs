use crate::execution_plan::ExecutionPlan;
use crate::metrics::MetricsAggregator;
use crate::scheduler::Scheduler;
use crate::vu_runner;
use crate::vu_runner::NextAction::{NotReady, Ready};
use crate::vu_runner::{Ctx, ExecutorAbstract, ExecutorMock, VUState, VuPool, VuRuntime};
use libprotocol::schema::Workload;
use serde::Serialize;
use std::collections::BTreeMap;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub(crate) struct RunEngine {

}

impl RunEngine {
    pub fn run_mock(plan: &ExecutionPlan, scenario: &libprotocol::schema::Scenario) -> RunReport {
        let vus = 65535;
        let start_time_ms = Instant::now();
        let scheduler: &mut Scheduler = &mut Scheduler::new(&scenario.workload);
        let workload: &Workload = &scenario.workload;
        let mut run_report = RunReport::new(scheduler);
        let mut metrics = MetricsAggregator::new();
        let sampler = &plan.weight_sampler;
        #[allow(dead_code)]
        let _runner_ctx = Ctx{};
        let mock_executor = ExecutorMock{};
        let runtime = VuRuntime{};
        let mut journey_per_vu: BTreeMap<usize, u64> = BTreeMap::new();
        let mut pool_vec = Vec::new();
        for i in 0..=vus.clone() {
            let stable_key = format!("{}-{}", i, sampler.seed);
            if let Some(journey_id) = sampler.peek(&*stable_key){
                // metrics, journey_per_vu
                journey_per_vu.entry(journey_id as usize).and_modify(|journey_count| *journey_count += 1).or_insert(1);

                pool_vec.push(VUState{
                    vu_id: i,
                    journey_id: journey_id as u32,
                    step_index: 0,
                    next_ready_at_ms: 0,
                    iteration_count: 0,
                    total_sleep_ms: 0,
                });
            };
        }
        let mut pool = VuPool::new(pool_vec);

        let mut total_ticks = 0;
        let mut missed_ticks = 0;
        let first_tick_ms = Instant::now();
        let mut last_tick_ms: Duration = Duration::from_millis(0);
        for tick in scheduler {
            total_ticks += 1;
            let start_tick = Instant::now();
            let now = tick.planned_at_ms;

            // Выравниваемся по времени тика
            if first_tick_ms.elapsed() < Duration::from_millis(tick.planned_at_ms) {
                sleep(Duration::from_millis(tick.planned_at_ms - (last_tick_ms.as_millis() as u64)));
            }

            let Some(vu_idx) = pool.pick_ready_vu(now) else {
                missed_ticks += 1;
                // take some relaxation
                sleep(Duration::from_millis(300));
                continue;
            };

            let vu = pool.get_mut(vu_idx).unwrap();

            match runtime.next_action(plan, vu, now) {
                NotReady(_next_ready_at) => { /* это баг pick_ready_vu */ missed_ticks += 1; run_report.vus.no_ready_ticks += 1; }
                vu_runner::NextAction::CompletedIteration => { /* no-op */ }
                Ready(req) => {
                    let res = mock_executor.execute(plan, &req, total_ticks).unwrap();
                    metrics.consume(res);
                    runtime.on_request_executed(plan, vu, now);
                }
            }
            last_tick_ms = start_tick.elapsed();
        }

        let scheduler: &mut Scheduler = &mut Scheduler::new(&scenario.workload);

        // Time
        run_report.time.real_time_duration_sec = start_time_ms.elapsed().as_secs();

        //Ticks arrival
        run_report.ticks_arrival.total = total_ticks;
        run_report.ticks_arrival.executed = metrics.total_requests;
        run_report.ticks_arrival.missed = missed_ticks;
        run_report.ticks_arrival.missed_ratio = missed_ticks as f64 / total_ticks as f64;

        // By endpoint
        run_report.by_endpoint = metrics.by_endpoint;

        // By journeys
        let by_journey = metrics.by_journey.iter().map(|(key, (journey_id, count))| {
            let per_vu = journey_per_vu.get(journey_id).unwrap_or(&0);
            ByJourney {
                id: *journey_id,
                key: key.clone(),
                per_vu: *per_vu,
                per_request: *count,
            }
        }).collect::<Vec<ByJourney>>();

        run_report.by_journey = by_journey;

        // RPS
        run_report.rps.planned_avg = workload.get_rps_avg() as u64;
        run_report.rps.achieved_avg = (metrics.total_requests as f64 / scheduler.planned_duration_sec as f64) as u64;

        // Error and quality
        run_report.error_and_quality.http_error_rate = (metrics.error_requests as f64 / metrics.total_requests as f64)*100.00_f64.round();


        // Vu Utilization
        run_report.vus.count = vus as u64;
        run_report.vus.no_ready_ratio = (run_report.vus.no_ready_ticks as f64 / total_ticks as f64).round();

        run_report.scenario = Scenario{ name: scenario.name.clone(), version: scenario.version.to_string() };

        run_report.run.total_ticks = total_ticks;
        run_report.run.duration_sec_planned = scheduler.planned_duration_sec as u64;

        run_report.requests.total = metrics.total_requests;
        run_report.requests.ok = metrics.ok_requests;
        run_report.requests.error = metrics.error_requests;
        run_report.latency_ms = LatencyMs{
            min: metrics.latency_min,
            max: metrics.latency_max,
            avg: metrics.latency_sum/metrics.total_requests,
        };

        run_report.sleep = pool.get_total_sleep_ms();

        run_report.missed_tick_count = missed_ticks as u16;

        run_report
    }
}

#[derive(Debug, Serialize)]
pub struct RunReport {
    scenario: Scenario,
    run: Run,
    ticks_arrival: TicksArrival,
    rps: Rps,
    journeys: Vec<Journey>,
    requests: Requests,
    latency_ms: LatencyMs,
    time: Time,
    missed_tick_count: u16,
    by_endpoint: BTreeMap<String, EndpointStats>,
    by_journey: Vec<ByJourney>,
    sleep: u64,
    error_and_quality: ErrorAndQuality,
    vus: VuUtilization
}


impl RunReport {
    pub fn new(scheduler: &Scheduler) -> Self {
    Self{
        scenario: Scenario { name: "".to_string(), version: "".to_string() },
        run: Run {
            mode: "".to_string(),
            seed: "".to_string(),
            total_ticks: 0,
            duration_sec_planned: 0,
        },
        ticks_arrival: TicksArrival {
            total: 0,
            executed: 0,
            missed: 0,
            missed_ratio: 0.0,
            tick_interval_ms: 0,
            first_tick_ms: 0,
            last_tick_ms: 0,
        },
        rps: Rps {
            planned_avg: 0,
            achieved_avg: 0,
            by_stage: 0,
        },
        journeys: vec![],
        requests: Requests {
            total: 0,
            ok: 0,
            error: 0,
        },
        latency_ms: LatencyMs {
            min: 0,
            max: 0,
            avg: 0,
        },
        time: Time {
            planned_start_ms: 0,
            planned_end_ms: 0,
            planned_duration_ms: scheduler.planned_duration_ms,
            planned_duration_sec: scheduler.planned_duration_sec,
            real_time_duration_sec: 0,
        },
        missed_tick_count: 0,
        by_endpoint: BTreeMap::new(),
        by_journey: vec![],
        sleep: 0,
        error_and_quality: ErrorAndQuality { http_error_rate: 0.0 },
        vus: VuUtilization { count: 0, no_ready_ticks: 0, no_ready_ratio: 0.0 }
    } }
}

#[derive(Debug, Serialize)]
pub struct VuUtilization {
    count: u64,
    no_ready_ticks: u64,
    no_ready_ratio: f64
}
#[derive(Debug, Serialize)]
pub struct EndpointStats {
    pub request: Requests,
    pub latency_ms: LatencyMs,
    pub achieved_rps: u64,
}

impl EndpointStats {
    pub(crate) fn default() -> EndpointStats {
        EndpointStats {
            request: Requests {
                total: 0,
                ok: 0,
                error: 0,
            },
            latency_ms: LatencyMs {
                min: 0,
                max: 0,
                avg: 0,
            },
            achieved_rps: 0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorAndQuality {
    http_error_rate: f64,
}
#[derive(Debug, Serialize)]
pub struct Time {
    pub planned_start_ms: u64,
    pub planned_end_ms: u64,
    pub planned_duration_ms: u64,
    pub planned_duration_sec: f64,
    pub real_time_duration_sec: u64
}

#[derive(Debug, Serialize)]
pub struct Rps {
    pub planned_avg: u64,
    pub achieved_avg: u64,
    pub by_stage: u64,
}

#[derive(Debug, Serialize)]
pub struct TicksArrival {
    pub total: u64,
    pub executed: u64,
    pub missed: u64,
    pub missed_ratio: f64,
    pub tick_interval_ms: u64,
    pub first_tick_ms: u64,
    pub last_tick_ms: u64
}
#[derive(Debug, Serialize)]
struct ByJourney {
    pub id: usize,
    pub key: String,
    pub per_vu: u64,
    pub per_request: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct LatencyMs {
    pub min: u64,
    pub max: u64,
    pub avg: u64
}

#[derive(Debug, Serialize)]
struct Scenario {
    name: String,
    version: String
}
#[derive(Debug, Serialize)]
struct Run {
    mode: String,
    seed: String,
    total_ticks: u64,
    duration_sec_planned: u64
}
#[derive(Debug, Serialize)]
struct Journey {
    name: String,
    weight: usize,
    picked: u32,
    share: String
}
#[derive(Debug, Serialize)]
pub(crate) struct Requests {
    pub total: u64,
    pub ok: u64,
    pub error: u64
}

#[cfg(test)]
mod tests {
    use crate::execution_plan::ExecutionPlan;
    use crate::run_engine::RunEngine;
    use libprotocol::Scenario;
    use std::path::PathBuf;

    #[test]
    fn it_run_mock_and_check_run_report() {
        let path = fixture_path("valid-extended-scenario-for_check_run_engine.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let scenario: Scenario = serde_json::from_str(&content).unwrap();
        let execution_plan = ExecutionPlan::from(&scenario);

        let mut report = RunEngine::run_mock(&execution_plan, &scenario);
        assert_eq!(true, report.time.real_time_duration_sec >= 3);
        report.time.real_time_duration_sec = 3; // flaky test
        insta::assert_debug_snapshot!(report);
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }
}