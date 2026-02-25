# rload

K6-like load testing engine/CLI written in Rust.

Pipeline: **Generate scenario → Setup → Validate → Run**  
Includes **dry-run** (plan preview / deterministic sampling) and **mock run** (no network) for fast testing.

> Status: MVP / work in progress. The core architecture is the focus.

## Key ideas

- Tick-based scheduler: workload stages (`duration_sec + rps`) → stream of ticks
- VU runtime executing journeys/steps
- Deterministic planning (seed) for reproducible runs
- JSON schema + scenario validation

## Workspace layout

- `bin/rload` — CLI binary
- `crates/libprotocol` — scenario schema + parsing/validation
- `crates/libruntime` — scheduler, VU runtime, metrics, run engine
- `crates/libcli` — CLI helpers / UI (WIP)
- `crates/test_support` — test utils / fixtures helpers

## Installation

### Build from source
```bash
cargo build --release
./target/release/rload --help
```
## Installation

Build from source:

```bash
cargo build --release
```
Run help:
```bash
./target/release/rload --help
```
---

## Usage

Run a scenario (real run):
```bash
./target/release/rload run --scenario examples/api-gw.json
```
Run mock (no network):
```bash
./target/release/rload run-mock --scenario examples/demo-scenario.json
```
Dry-run (plan preview / deterministic):
```bash
./target/release/rload dry-run --scenario examples/demo-scenario.json --iterations 1000
```
Useful dry-run options:
--seed <SEED> (default: 1000)
--print-plan
--limit-steps <N>
--output json
--is-simulated

---

## Scenario validation

Validate scenario:
```bash
./target/release/rload validate --scenario examples/demo-scenario.json
```
Export JSON schema:
```bash
./target/release/rload schema --path docs/schema.json --version 1
```
Generate scenario template:
```bash
./target/release/rload generate --path examples/generated.json --version 1
```
---

---

## Report (JSON example)

After `run` or `run-mock`, the engine produces a structured JSON report.

Example:
```json
{
  "scenario": {
    "name": "default_scenario",
    "version": "1"
  },
  "run": {
    "total_ticks": 1000,
    "duration_sec_planned": 10
  },
  "rps": {
    "planned_avg": 100,
    "achieved_avg": 111
  },
  "requests": {
    "total": 1000,
    "ok": 1000,
    "error": 0
  },
  "latency_overall_summary": {
    "count": 1000,
    "p50": 0,
    "p95": 0,
    "p99": 0
  },
  "error_and_quality": {
    "http_error_rate": 0.0
  }
}
```
### Full report includes:
* tick arrival stats
* per-stage metrics
* per-endpoint metrics
* per-journey stats
* latency percentiles
* VU behavior metrics
* time model metrics

### Designed for:
* CI validation
* dashboards
* regression testing
* threshold evaluation (planned)

## Roadmap

See ROADMAP.md

---

## License

MIT