use std::any::{Any, TypeId};
use std::cmp::PartialEq;
use std::thread::sleep;
use std::time::Duration;
use sha2::{Digest, Sha256};
use libprotocol::schema::Step::{Request, Sleep};
use libprotocol::schema::StepMethod;
use crate::execution_plan::ExecutionPlan;
use crate::vu_runner::NextAction::NotReady;

pub struct VuPool {
    vus: Vec<VUState>,
    cursor: usize,
}

impl VuPool {
    pub fn new(vus: Vec<VUState>) -> Self {
        VuPool { vus, cursor: 0 }
    }
    pub fn pick_ready_vu(&self, now_ms: u64) -> Option<usize> {
        for (idx, vu) in self.vus.iter().enumerate() {
            if vu.next_ready_at_ms <= now_ms {
                return Some(idx);
            }
        }
        None
    }
    pub fn get_total_sleep_ms(&self) -> u64 {
        self.vus.iter().map(|vu| vu.total_sleep_ms).sum()
    }
    pub fn get_mut(&mut self, vu_idx: usize) -> Result<&mut VUState, String> {
        let vu = self.vus.get_mut(vu_idx).ok_or("VU index out of bounds".parse().unwrap());
        vu
    }
}

pub struct VUState{
    pub vu_id: u32,
    pub journey_id: u32,
    pub step_index: usize, // (на каком шаге стоим)
    pub next_ready_at_ms: u64, // (когда VU “готов” к следующему request после sleeps)
    pub iteration_count: u64, // (сколько раз завершили journey и начали заново) — опционально
    pub total_sleep_ms: u64 // сколько эта vu спала
}

pub struct Ctx {

}

struct VuIterationStats {
    sleep_steps: u64,
    request_steps: u64,
    sleep_ms_total: u64,
    requests: Vec<ResponseResult>
}

// pub(crate) struct RequestEvent {
//     pub(crate) journey_name: String,
//     pub(crate) endpoint_key: String,
//     ok: bool,
//     pub(crate) latency_ms: u32,
//     error_kind: Option<ErrorType>
// 
// }

pub struct RequestSpec {
    pub(crate) method: StepMethod,
    pub path: String,
    pub endpoint_key: String,
    pub timeout_ms: u64,
    pub journey_id: u64
}

pub struct VuRuntime {

}

pub enum NextAction {
    NotReady(u64),
    Ready(RequestSpec),
    CompletedIteration
}

impl PartialEq for NextAction {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl VuRuntime {
    pub fn on_request_executed(&self, plan: &ExecutionPlan, vu: &mut VUState, now_ms: u64) {
        vu.step_index +=1;
        vu.next_ready_at_ms += now_ms;
        let steps = plan.get_journey(vu.journey_id as i32).steps.clone();

        if steps.len() <= vu.step_index {
            vu.step_index = 0;
            vu.iteration_count +=1
        }
    }

    pub fn next_action(&self, plan: &ExecutionPlan, vu: &mut VUState, now_ms: u64) -> NextAction {
        if vu.next_ready_at_ms > now_ms {
            return NotReady(vu.next_ready_at_ms)
        }

        let steps = plan.get_journey(vu.journey_id as i32).steps.clone();

        if steps.len() <= vu.step_index {
            vu.step_index = 0;
            vu.iteration_count +=1
        }

        let pick_step = |idx: usize| {
            steps.get(idx)
        };
        let mut next_action: NextAction = NextAction::CompletedIteration;
        while let Some(step) = pick_step(vu.step_index) {
            next_action = match step {
                Sleep{ duration_ms } => {
                    let base = vu.next_ready_at_ms.max(now_ms);
                    vu.next_ready_at_ms = base + *duration_ms as u64;
                    vu.step_index +=1;
                    vu.total_sleep_ms += *duration_ms as u64;
                    continue;
                },
                Request { method, path, headers, body, timeout_ms } => {
                    NextAction::Ready(RequestSpec {
                        method: *method,
                        path: path.clone(),
                        endpoint_key: format!("{:?}-{:?}", method, path).to_string(),
                        timeout_ms: timeout_ms.unwrap_or(0) as u64,
                        journey_id: vu.journey_id as u64,
                    })
                }
            };

            if matches!(next_action, NextAction::Ready(..)) {
                break;
            }
        }
        next_action
    }

    pub fn run_one_iteration(vu_id: u32, journey: libprotocol::schema::Journey, ctx: Ctx, executor: Box<dyn ExecutorAbstract>) -> VuIterationStats {

        VuIterationStats {
            sleep_steps: 0,
            request_steps: 0,
            sleep_ms_total: 0,
            requests: Vec::new()
        }
    }
}


pub trait ExecutorAbstract{
    fn execute(&self, plan: &ExecutionPlan, request: &RequestSpec, tick_ids: u64) -> Result<ResponseResult, String>;
}

pub struct ExecutorMock {

}
impl ExecutorAbstract for ExecutorMock {
    fn execute(&self, plan: &ExecutionPlan, request: &RequestSpec, tick_idx: u64) -> Result<ResponseResult, String> {
        let stable_key = format!("{}-{}-{}", request.path, plan.scenario_name, tick_idx);
        let latency_hash = Sha256::digest(stable_key.as_bytes());
        let first8: [u8; 8] = latency_hash[0..8].try_into().unwrap();
        let n = u64::from_be_bytes(first8);


        let latency_ms =  n % 100 as u64;
        // emulate latency for tests
        // sleep(Duration::from_millis(latency_ms));
        Ok(ResponseResult {
            ok: true,
            latency_ms,
            error_kind: None,
            endpoint_key: request.endpoint_key.clone(),
            journey_name: plan.scenario_name.clone(),
            journey_id: request.journey_id,
        })
    }
}

pub struct ResponseResult {
    pub ok: bool,
    pub(crate) latency_ms: u64,
    pub error_kind: Option<ErrorType>,
    pub(crate) endpoint_key: String,
    pub journey_name: String,
    pub journey_id: u64
}

enum ErrorType {
    Timeout,
    ConnectionError
}