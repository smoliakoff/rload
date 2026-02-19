use libprotocol::schema::{Stage, Workload};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;

pub struct Scheduler {
    stages: Vec<Stage>,
    current_index: u64,
    current_stage_index: usize,
    current_step_index: usize,
    #[allow(dead_code)]
    start_time: u64,
    stage_max_ticks: HashMap<usize, i32>,
    total_time: u64,
    pub(crate) planned_duration_ms: u64,
    pub(crate) planned_duration_sec: f64,
}

impl Scheduler {
    pub(crate) fn new(workload: &Workload) -> Self {
        let stage_max_ticks = workload.stages.iter().enumerate()
            .map(|(i, stage)| (i, stage.duration_sec * stage.rps)).collect();
        let planned_duration_ms: i32 = workload.stages.iter()
            .map(| stage| stage.duration_sec * 1000).sum();
        let planned_duration_sec: i32 = workload.stages.iter()
            .map(|stage|  stage.duration_sec).sum();

        Scheduler {
            stages: workload.stages.clone(),
            current_index: 0,
            current_stage_index: 0,
            current_step_index: 0,
            start_time: 0,
            stage_max_ticks,
            planned_duration_ms: planned_duration_ms as u64,
            planned_duration_sec: planned_duration_sec as f64,
            total_time: 0,
        }
    }
    pub fn get_stage_max_ticks(&self, stage_index: usize) -> Option<i32> {
        self.stage_max_ticks.get(&stage_index).copied()
    }
}
impl Iterator for &mut Scheduler {
    type Item = Tick;

    fn next(&mut self) -> Option<Self::Item> {

        if let Some(max) = self.get_stage_max_ticks(self.current_stage_index) {
            if self.current_step_index >= max as usize {
                self.current_step_index = 0;
                self.current_stage_index+=1;
            }
        }

        if self.current_stage_index >= self.stages.len() {
            return None;
        }

        let stage = self.stages.get(self.current_stage_index).expect("stage not found");

        let current_tick_delay = 1000 / stage.rps;
        self.total_time += current_tick_delay as u64;
        let planed_at_ms = self.total_time;

        self.current_index+=1;
        self.current_step_index+=1;
        Some(Tick{
            tick_index: self.current_index,
            stage_index: self.current_stage_index as u64,
            planned_at_ms: planed_at_ms,
            target_rps: stage.rps as u32,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tick {
    pub tick_index: u64,
    pub stage_index: u64,
    pub planned_at_ms: u64,
    pub target_rps:u32
}

#[cfg(test)]
mod tests {
    use crate::scheduler::{Scheduler, Tick};
    use libprotocol::schema::{Stage, Workload};

    #[test]
    fn it_one_stage_2sec_5rps() {
        let workload = Workload {
            stages: vec!(
                Stage {
                    duration_sec: 2,
                    rps: 5,
                },
                Stage {
                    duration_sec: 1,
                    rps: 3,
                }),
        };
        let mut ticks: Vec<Tick> = Vec::new();
        let scheduler: &mut Scheduler = &mut Scheduler::new(&workload);
        for tick in scheduler.into_iter() {
        ticks.push(tick)
        }
        insta::assert_debug_snapshot!(ticks)
    }
    #[test]
    fn it_planned_time_growing() {
        let workload = Workload {
            stages: vec!(
                Stage {
                    duration_sec: 2,
                    rps: 5,
                })
        };
        let scheduler: &mut Scheduler = &mut Scheduler::new(&workload);
        let delta = 200;
        let mut prev_planned_time = 0;
        for tick in &mut scheduler.into_iter() {

            assert_eq!(delta, tick.planned_at_ms - prev_planned_time);
            prev_planned_time = tick.planned_at_ms
        }
    }
}
