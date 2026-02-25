# Architecture overview

`rload` is a Rust workspace split into clear layers.

## High-level flow

Scenario file → libprotocol → execution plan → libruntime run engine → metrics/report

1) **libprotocol**
- Owns scenario schema (v1)
- JSON Schema export
- Semantic validation
- Parsing into strongly typed structures

2) **Execution planning (libruntime)**
- Builds `ExecutionPlan` from scenario:
    - workload stages (duration/rps)
    - journeys (weights)
    - steps
- Uses deterministic **weighted sampler** (seed) to choose journeys

3) **Scheduler**
- Converts workload stages into a stream of **Tick** events:
    - tick_index
    - stage_index
    - planned_at_ms (time model)
    - target_rps
- Streams ticks (does not pre-generate huge arrays)

4) **VU Runtime**
- Maintains VU state (journey_id, step_index, next_ready_at_ms, iteration_count)
- On each tick, decides next action:
    - sleep
    - request (via executor)
    - iteration completed

5) **Executors**
- `run-mock`: no network, fast deterministic tests
- `run`: real HTTP (WIP / evolving)

6) **Metrics & Report**
- Aggregates counters + timings
- Produces a JSON report for external consumption (dashboards, CI, etc.)

## Design goals
- Deterministic mode for reproducible runs
- Clear module boundaries and testability
- “k6-like” ergonomics (thresholds + exit codes + live stats)
- Extensibility and modularity for future features (e.g., new executors, metrics backends)