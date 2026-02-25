use crate::execution_plan::ExecutionPlan;
use crate::metrics::MetricsAggregator;
use crate::scheduler::Scheduler;
use crate::vu_runner;
use crate::vu_runner::NextAction::{NotReady, Ready};
use crate::vu_runner::{Ctx, ErrorType, ExecutorAbstract, ExecutorHttp, ExecutorMock, ResponseResult, VUState, VuPool, VuRuntime};
use libprotocol::schema::Workload;
use serde::{Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use hdrhistogram::Histogram;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::Instant;
use crate::events::{Event, EventSink};

pub const LOWEST_US: u64 = 1;
pub const HIGHEST_US: u64 = 60_000_000;
pub const SIGFIG: u8 = 3;

pub(crate) struct RunEngine {
    pub is_mock: bool,
    pub is_real_time: bool
}

pub enum RunMode {
    Real,
    Deterministic,
}

struct Completed {
    vu_idx: usize,
    now_ms: u64,
    res: ResponseResult,
    last_request_started_ms: u64
}

impl RunEngine {
    pub fn new(is_mock: Option<bool>, is_real_time: Option<bool>) -> Self {
        Self { is_mock: is_mock.unwrap_or(false), is_real_time: is_real_time.unwrap_or(true) }
    }

    pub async fn run(&self, plan: &ExecutionPlan, scenario: &libprotocol::schema::Scenario, sink: EventSink<Event>) -> RunReport {
        let vus = 1000;
        let start_time_ms = tokio::time::Instant::now();
        let mut first_tick_real_ms: Option<u64> = None;
        let mut last_tick_real_ms: u64 = 0;

        let mode = match self.is_real_time {
            false => RunMode::Deterministic,
            true => RunMode::Real,
        };

        let scheduler: &mut Scheduler = &mut Scheduler::new(&scenario.workload);
        let mut run_report = RunReport::new(scheduler);

        let planned_duration_ms = scheduler.planned_duration_ms;

        let workload: &Workload = &scenario.workload;
        let mut metrics = MetricsAggregator::new();
        let sampler = &plan.weight_sampler;
        #[allow(dead_code)]
        let _runner_ctx = Ctx{};

        let executor_instance: Box<dyn ExecutorAbstract> = match self.is_mock {
            true => ExecutorMock::new_instance(),
            false => ExecutorHttp::new_instance()
        };
        let executor = Arc::new(executor_instance);
        let plan = Arc::new(plan.clone());

        let runtime = VuRuntime{};
        let mut journey_per_vu: BTreeMap<usize, u64> = BTreeMap::new();
        let mut pool_vec = Vec::new();
        for i in 0..=vus {
            let stable_key = format!("{}-{}", i, sampler.seed);
            if let Some(journey_id) = sampler.peek(&stable_key){
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
        let mut first_tick_ms = tokio::time::Instant::now();
        run_report.ticks_arrival.first_tick_ms = first_tick_ms.elapsed().as_millis() as u64;

        let max_in_flight = 1000usize;
        let sem = Arc::new(Semaphore::new(max_in_flight));

        let (tx, mut rx) = mpsc::unbounded_channel::<Completed>();

        let stop_at_ms = planned_duration_ms;
        let mut handles = Vec::new();

        // Real-time origin
        let origin = Instant::now();
        // ---- MAIN LOOP ----
        for tick in scheduler {
            sink.send(Event::TickExecuted{tick});
            let planned_now = tick.planned_at_ms;

            // 1) Time alignment / virtual time
            let now = match mode {
                RunMode::Real => {
                    let elapsed = origin.elapsed().as_millis() as u64;
                    if planned_now > elapsed {
                        tokio::time::sleep(Duration::from_millis(planned_now - elapsed)).await;
                    }
                    origin.elapsed().as_millis() as u64
                }
                RunMode::Deterministic => {
                    // virtual time = planned
                    planned_now
                }
            };

            if first_tick_real_ms.is_none() {
                first_tick_real_ms = Some(first_tick_ms.elapsed().as_millis() as u64);
                first_tick_ms = Instant::now();
            }

            total_ticks += 1;

            // 1) Alignment with planned time
            let elapsed_ms = first_tick_ms.elapsed().as_millis() as u64;
            if tick.planned_at_ms > elapsed_ms {
                tokio::time::sleep(Duration::from_millis(tick.planned_at_ms - elapsed_ms)).await;
            }

            last_tick_real_ms = now;
            if now > stop_at_ms + 1 {
                break; // window finished — no new requests started
            }

            // 2) read from channel
            while let Ok(done) = rx.try_recv() {
                sink.send(Event::RequestFinished{ok: done.res.ok, latency_ms: done.res.latency_ms as u32 });
                metrics.consume(done.res, done.last_request_started_ms);
                let vu = pool.get_mut(done.vu_idx).unwrap();
                runtime.on_request_executed(&plan, vu, done.now_ms);
            }

            // 3) pick VU
            let Some(vu_idx) = pool.pick_ready_vu(now) else {
                missed_ticks += 1;
                // take some relaxation
                tokio::time::sleep(Duration::from_millis(300)).await;
                continue;
            };

            let vu = pool.get_mut(vu_idx).unwrap();

            match runtime.next_action(&plan, vu, now).await {
                NotReady(_next_ready_at) => { /* possible bug pick_ready_vu */ missed_ticks += 1; run_report.vus.no_ready_ticks += 1; }
                vu_runner::NextAction::CompletedIteration => { /* no-op */ }
                Ready(mut req) => {
                    req.stage_index = tick.stage_index;

                    match mode {
                        RunMode::Deterministic => {
                            // INLINE EXECUTION (no spawn, no sem, no tx)
                            let started_ms = planned_now;
                            let mut res = executor
                                .execute(&plan, &req, total_ticks)
                                .await
                                .unwrap_or_else(|e| {
                                    eprintln!("executor error: {e}");
                                    let stage_start_ms = match tick.is_new_stage {
                                        true => planned_now,
                                        false => 0,
                                    };
                                    ResponseResult {
                                        ok: false,
                                        latency_ms: 0,
                                        latency_us: 0,
                                        error_kind: Some(ErrorType::ConnectionError),
                                        endpoint_key: req.endpoint_key.clone(),
                                        journey_name: "".to_string(),
                                        journey_id: req.journey_id,
                                        stage_index: tick.stage_index,
                                        stage_start_ms,
                                    }
                                });

                            if tick.is_new_stage {
                                res.stage_start_ms = planned_now;
                            }

                            let finished_ms = started_ms + res.latency_ms;

                            metrics.consume(res, started_ms);
                            let vu = pool.get_mut(vu_idx).unwrap();
                            runtime.on_request_executed(&plan, vu, finished_ms);
                        }

                        RunMode::Real => {
                            // ASYNC EXECUTION (spawn + sem + tx)
                            let permit = sem.clone().acquire_owned().await.unwrap();
                            let last_request_started_ms = origin.elapsed().as_millis() as u64;

                            let tx = tx.clone();
                            let executor = executor.clone();
                            let plan = plan.clone();
                            let start_request = Instant::now();

                            let stage_index = tick.stage_index;
                            let sink_clone = sink.clone();
                            let handle = tokio::spawn(async move {
                                let _permit = permit;

                                let mut res = executor.execute(&plan, &req, total_ticks).await
                                    .unwrap_or_else(|e| {
                                        eprintln!("executor error: {e}");
                                        let stage_start_ms = match tick.is_new_stage {
                                            true => planned_now,
                                            false => 0,
                                        };
                                        sink_clone.send(Event::RequestFinished{ok: false, latency_ms: start_request.elapsed().as_millis() as u32 });

                                        ResponseResult {
                                            ok: false,
                                            latency_ms: 0,
                                            latency_us: 0,
                                            error_kind: Some(ErrorType::ConnectionError),
                                            endpoint_key: req.endpoint_key.clone(),
                                            journey_name: "".to_string(),
                                            journey_id: req.journey_id,
                                            stage_index,
                                            stage_start_ms,
                                        }
                                    });

                                if tick.is_new_stage {
                                    res.stage_start_ms = last_request_started_ms;
                                }

                                let finished_ms = last_request_started_ms.saturating_add(res.latency_ms);

                                let _ = tx.send(Completed {
                                    vu_idx,
                                    now_ms: finished_ms,
                                    res,
                                    last_request_started_ms,
                                });
                            });

                            handles.push(handle);
                        }
                    }
                }
            }
        }
        // ---- END LOOP ----

        let start_drain = tokio::time::Instant::now();

        // REAL-ONLY: grace + join + final rx drain
        if matches!(mode, RunMode::Real) {
            let grace_ms = 10_000;
            let drain = sem.acquire_many_owned(max_in_flight as u32);
            if tokio::time::timeout(Duration::from_millis(grace_ms), drain).await.is_err() {
                for h in &handles {
                    h.abort();
                }
            }

            for h in handles {
                let _ = h.await;
            }

            while let Ok(done) = rx.try_recv() {
                sink.send(Event::RequestFinished{ok: done.res.ok, latency_ms: done.res.latency_ms as u32 });
                metrics.consume(done.res, done.last_request_started_ms);
                let vu = pool.get_mut(done.vu_idx).unwrap();
                runtime.on_request_executed(&plan, vu, done.now_ms);
            }
        }

        sink.send(Event::RunFinished);

        let drain_time = start_drain.elapsed().as_secs();
        let scheduler: &mut Scheduler = &mut Scheduler::new(&scenario.workload);

        // Time
        run_report.time.real_time_duration_sec = start_time_ms.elapsed().as_secs();

        //Ticks arrival
        run_report.ticks_arrival.first_tick_ms = first_tick_real_ms.expect("first tick not never arrived");
        run_report.ticks_arrival.last_tick_ms = last_tick_real_ms;
        run_report.ticks_arrival.total = total_ticks;
        run_report.ticks_arrival.executed = metrics.total_requests;
        run_report.ticks_arrival.missed = missed_ticks;
        run_report.ticks_arrival.missed_ratio = missed_ticks as f64 / total_ticks as f64;
        if total_ticks > 1 {
            let total_real_interval = last_tick_real_ms - first_tick_real_ms.unwrap();
            run_report.ticks_arrival.tick_interval_ms =
                total_real_interval / (total_ticks - 1);
        }

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
        run_report.rps.achieved_avg_including_drain = (metrics.total_requests as f64 / run_report.time.real_time_duration_sec as f64) as u64;
        run_report.rps.achieved_avg = (metrics.total_requests as f64 / run_report.time.real_time_duration_sec as f64 - drain_time as f64 / run_report.time.real_time_duration_sec as f64) as u64;
        run_report.rps.by_stage = metrics.by_stage;
        // Error and quality
        run_report.error_and_quality.http_error_rate = ((metrics.error_requests as f64 / metrics.total_requests as f64)*100.00_f64).round();

        // Vu Utilization
        run_report.vus.count = vus as u64;
        run_report.vus.no_ready_ratio = (run_report.vus.no_ready_ticks as f64 / total_ticks as f64).round();

        run_report.scenario = Scenario{ name: scenario.name.clone(), version: scenario.version.to_string() };

        run_report.run.total_ticks = total_ticks;
        run_report.run.duration_sec_planned = scheduler.planned_duration_sec as u64;

        run_report.requests.total = metrics.total_requests;
        run_report.requests.ok = metrics.ok_requests;
        run_report.requests.error = metrics.error_requests;

        // Latency by stage
        run_report.latency_by_stage = metrics.latency_by_stage.iter()
            .map(|(stage_index, hist)| (*stage_index, LatencySummary::summarize(hist))).collect();

        // Latency by endpoint
        for (key, rec) in run_report.by_endpoint.iter_mut() {
            let lat_summ = metrics
                .latency_by_endpoint
                .get(key)
                .expect("histogram missing for endpoint");

            rec.latency_summary = LatencySummary::summarize(lat_summ);
        }


        run_report.latency_overall_summary = LatencySummary::summarize(&metrics.overall_latency);

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
    latency_overall_summary: LatencySummary,
    latency_by_stage: BTreeMap<u64, LatencySummary>,
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
        run: Default::default(),
        ticks_arrival: Default::default(),
        rps: Default::default(),
        journeys: vec![],
        requests: Default::default(),
        latency_overall_summary: Default::default(),
        latency_by_stage: Default::default(),
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
pub struct ByStage {
    pub stage_index: u64,
    pub achieved_rps: u64,
    pub request_count: u64,
    pub stage_started_ms: u64,
    pub stage_duration_ms: u64,
}
impl Default for ByStage {
    fn default() -> ByStage {
        ByStage {
            stage_index: 0,
            achieved_rps: 0,
            request_count: 1,
            stage_started_ms: 0,
            stage_duration_ms: 0,
        }
    }
}
#[derive(Debug, Serialize, Copy, Clone)]
pub struct EndpointStats {
    pub request: Requests,
    pub latency_summary: LatencySummary,
    pub achieved_rps: f64,
    pub first_at_ms: u64,
    pub last_at_ms: u64,
    pub count: u64
}
impl EndpointStats {
    pub(crate) fn default() -> EndpointStats {
        EndpointStats {
            request: Requests {
                total: 0,
                ok: 0,
                error: 0,
            },
            latency_summary: Default::default(),
            achieved_rps: 0.0,
            first_at_ms: 0,
            last_at_ms: 0,
            count: 0,
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
#[derive(Debug, Serialize, Default)]
pub struct Rps {
    pub planned_avg: u64,
    pub achieved_avg: u64,
    pub achieved_avg_including_drain: u64,
    pub by_stage: BTreeMap<u64, ByStage>,
}
#[derive(Debug, Serialize, Default)]
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

#[derive(Debug, Serialize, Copy, Clone, Default)]
pub struct LatencySummary { count: u64, min: u64, max: u64, mean: u64, p50: u64, p90: u64, p95: u64, p99: u64 }

impl LatencySummary {
    pub fn summarize(histogram: &Histogram<u64>) -> Self {
        LatencySummary{
            count: histogram.len(),
            min: histogram.min()/1000,
            max: histogram.max()/1000,
            mean: histogram.mean() as u64/1000,
            p50: histogram.value_at_quantile(0.50)/1000,
            p90: histogram.value_at_quantile(0.90)/1000,
            p95: histogram.value_at_quantile(0.95)/1000,
            p99: histogram.value_at_quantile(0.99)/1000,
        }
    }
}

#[derive(Debug, Serialize)]
struct Scenario {
    name: String,
    version: String
}
#[derive(Debug, Serialize, Default)]
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
#[derive(Debug, Serialize, Copy, Clone, Default)]
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
    use crate::events::{Event, EventSink};

    #[tokio::test]
    async fn it_run_mock_and_check_run_report() {
        let path = fixture_path("valid-extended-scenario-for_check_run_engine.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let scenario: Scenario = serde_json::from_str(&content).unwrap();
        let execution_plan = ExecutionPlan::from(&scenario);
        let sink = EventSink::<Event>::noop();

        let mut report = RunEngine::new(Some(true), Some(false)).run(&execution_plan, &scenario, sink).await;
        report.time.real_time_duration_sec = 3; // flaky test
        insta::assert_debug_snapshot!(report);
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    }

    #[tokio::test]
    #[ignore] // Non deterministic
    async fn it_run_and_check_run_report() {
        let (base_url, shutdown_tx, handle) = test_support::test_server::spawn_test_server();

        let path = fixture_path("valid-extended-scenario-for_check_with_real_http.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let scenario: Scenario = serde_json::from_str(&content).unwrap();
        let mut execution_plan = ExecutionPlan::from(&scenario);
        execution_plan.base_url = base_url.clone();
        let sink = EventSink::<Event>::noop();

        let mut report = RunEngine::new(Some(false), Some(true)).run(&execution_plan, &scenario, sink).await;
        report.time.real_time_duration_sec = 1; // flaky test
        insta::assert_debug_snapshot!(report);
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        // shutdown
        let _ = shutdown_tx.send(());
        let _ = handle.await;
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }
}