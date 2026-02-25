use libprotocol::schema::{Stage, Workload};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;

pub struct Scheduler {
    pub total_ticks: u64,
    stages: Vec<Stage>,
    current_stage_index: usize,
    current_step_index: usize,
    stage_offset_ms: u64,
    stage_start_ms: u64,
    stage_max_ticks: HashMap<usize, i32>,
    pub(crate) planned_duration_ms: u64,
    pub(crate) planned_duration_sec: f64,
}

impl Scheduler {
    pub fn new(workload: &Workload) -> Self {
        let stage_max_ticks: HashMap<usize, i32> = workload.stages.iter().enumerate()
            .map(|(i, stage)| (i, stage.duration_sec * stage.rps)).collect();
        let planned_duration_ms: i32 = workload.stages.iter()
            .map(| stage| stage.duration_sec * 1000).sum();
        let planned_duration_sec: i32 = workload.stages.iter()
            .map(|stage|  stage.duration_sec).sum();

        let total_ticks: u64 = stage_max_ticks.values().sum::<i32>() as u64;

        Scheduler {
            stages: workload.stages.clone(),
            current_stage_index: 0,
            current_step_index: 0,
            stage_offset_ms: 0,
            stage_start_ms: 0,
            stage_max_ticks,
            total_ticks,
            planned_duration_ms: planned_duration_ms as u64,
            planned_duration_sec: planned_duration_sec as f64,
        }
    }
    pub fn get_stage_max_ticks(&self, stage_index: usize) -> Option<i32> {
        self.stage_max_ticks.get(&stage_index).copied()
    }
}
impl Iterator for &mut Scheduler {
    type Item = Tick;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(max) = self.get_stage_max_ticks(self.current_stage_index) &&
            self.current_step_index >= max as usize {
                // добавляем длительность завершённого stage в offset
                let finished_stage = &self.stages[self.current_stage_index];
                self.stage_offset_ms += finished_stage.duration_sec as u64 * 1000;

                // move on to the next one
                self.current_step_index = 0;
                self.current_stage_index += 1;

                // start of new stage = current offset
                self.stage_start_ms = self.stage_offset_ms;
        }
        let is_new_stage = self.current_step_index == 0;

        if self.current_stage_index >= self.stages.len() {
            return None;
        }

        let stage = self.stages.get(self.current_stage_index).expect("stage not found");

        let tick_in_stage = self.current_step_index as u64;

        self.current_step_index+=1;
        let tick_offset_ms = tick_in_stage * 1000 / (stage.rps as u64);

        Some(Tick{
            tick_index: tick_in_stage,
            stage_index: self.current_stage_index as u64,
            planned_at_ms: self.stage_start_ms + tick_offset_ms,
            target_rps: stage.rps as u32,
            is_new_stage,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Tick {
    pub tick_index: u64,
    pub stage_index: u64,
    pub planned_at_ms: u64,
    pub target_rps:u32,
    pub is_new_stage: bool
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
                },
                Stage {
                    duration_sec: 2,
                    rps: 5,
                })
        };
        let scheduler: &mut Scheduler = &mut Scheduler::new(&workload);
        let delta = 200;
        let mut prev_planned_time = 0;
        for (i, tick) in &mut scheduler.into_iter().enumerate() {
            if i != 0 {
                assert_eq!(delta, tick.planned_at_ms - prev_planned_time);
            }
            prev_planned_time = tick.planned_at_ms
        }
    }
}
