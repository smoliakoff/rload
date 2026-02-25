mod ui;
mod stats;

use clap::{Parser, Subcommand};
use console::style;
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::mpsc;
use libruntime::events::{Event, EventSink};
use libruntime::scheduler::Scheduler;
use crate::stats::live::LiveStats;

/// A fictional versioning CLI
#[derive(Debug, Parser)]
#[command(name = "lt_engine")]
#[command(about = "Load Testing. Generate scenario -> SetUp -> Validate -> Run it !", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run a scenario
    #[command(arg_required_else_help = true)]
    Run{
        #[arg(
            short,
            long,
            required = true,
            require_equals = true,
        )]
        scenario: String,
    },
    /// Run mock a scenario
    #[command(arg_required_else_help = true)]
    RunMock {
        #[arg(
            short,
            long,
            required = true,
            require_equals = true,
        )]
        scenario: String,
    },
    /// Dry-run a scenario
    #[command(arg_required_else_help = true)]
    DryRun {
        #[arg(
            short,
            long,
            required = true,
            require_equals = true,
        )]
        scenario: String,
        #[arg(
            short,
            long,
            default_missing_value = "1000"
        )]
        iterations: u32,
        #[arg(
            short = 'e',
            long,
            default_value_t = 1000u32
        )]
        seed: u32,
        #[arg(
            short,
            long,
            default_value_t = String::from("json")
        )]
        output: String,
        #[arg(
            short = 'm',
            long,
            default_value_t = false
        )]
        is_simulated: bool,
        #[arg(
            short,
            long,
            default_value_t = 1000u32
        )]
        limit_steps: u32,
        /// Print plan to console
        #[arg(
            short,
            long,
            require_equals = false,
        )]
        print_plan: bool,

    },
    #[command(arg_required_else_help = false)]
    Generate {
        #[arg(
            long,
            require_equals = true,
            default_missing_value = "./demo-scenario.json"
        )]
        path: Option<String>,
        #[arg(
            long,
            require_equals = false,
            default_missing_value = "1"
        )]
        version: Option<String>,

    },
    /// Export a json schema for scenario
    #[command(arg_required_else_help = false)]
    Schema {
        #[arg(
            long,
            require_equals = true,
            default_missing_value = "./demo-scenario.json"
        )]
        path: Option<String>,
        #[arg(
            long,
            require_equals = false,
            default_missing_value = "1"
        )]
        version: Option<String>
    },
    /// Validate given scenario
    #[command(arg_required_else_help = true)]
    Validate {
        #[arg(
            long,
            required = true,
            require_equals = true,
        )]
        scenario: Option<String>,
    },
}

pub async fn run() -> anyhow::Result<()>{
    let args = Cli::parse();

    match args.command {
        Commands::Generate { path, version } => {
            libprotocol::generate_scenario(path.unwrap_or("./demo-scenario.json".to_string()), &version.unwrap_or_else(|| "1".to_string()))
        },
        Commands::Schema { path, version } => {
            libprotocol::export_schema(path.unwrap_or("./schema.json".to_string()), version)
        },
        Commands::Validate { scenario } => {
            Ok(libprotocol::validate(scenario.unwrap())?)
        },
        Commands::DryRun { scenario, seed, iterations, is_simulated, .. } => {
            let (tx, _rx) = mpsc::unbounded_channel();
            let sink = EventSink::new(tx);

            libruntime::dry_run(scenario, seed, iterations, is_simulated, sink).await;
            Ok(())
        },
        Commands::RunMock { scenario} => {
            let (tx, _rx) = mpsc::unbounded_channel();
            let sink = EventSink::new(tx);

            libruntime::run(scenario, Some(true), sink).await;
            Ok(())
        },
        Commands::Run { scenario} => {
            let scenario_instance = &libprotocol::parse_scenario(&scenario);
            let scheduler: Scheduler = Scheduler::new(&scenario_instance.workload);

            let (tx, rx) = mpsc::unbounded_channel();
            let sink = EventSink::new(tx);

            let total_ticks = scheduler.total_ticks;
            //
            print_banner(env!("CARGO_PKG_VERSION"));
            println!("OS: {}  CPU: {}",
                     std::env::consts::OS,
                     num_cpus::get()
            );
            println!("\n\n");

            let ui = tokio::spawn(async move {
                ui_task(rx, total_ticks).await;
            });

            let report = libruntime::run(scenario, Option::from(false), sink).await;

            ui.await.ok();
            Ok(report)
        },

    }
}

async fn ui_task(
    mut rx: mpsc::UnboundedReceiver<Event>,
    total_ticks: u64,
) {
    let pb = ProgressBar::new(total_ticks);
    pb.set_style(
        ProgressStyle::with_template("[{bar:60}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut s = LiveStats::default();

    // таймер обновления UI
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(250));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                pb.set_position(s.totals.ticks_executed);
                let err_rate = if (s.totals.requests_ok + s.totals.requests_err) > 0 {
                    (s.totals.requests_err as f64) / ((s.totals.requests_ok + s.totals.requests_err) as f64) * 100.0
                } else { 0.0 };

                pb.set_message(format!(
                    "ok={} err={} ({:.2}%) in_flight={}",
                    s.totals.requests_ok, s.totals.requests_err, err_rate, s.totals.in_flight
                ));
            }
            ev = rx.recv() => {
                match ev {
                    Some(Event::TickExecuted{..}) => s.totals.ticks_executed += 1,
                    Some(Event::RequestFinished{ok, latency_ms: _}) => {
                        if ok { s.totals.requests_ok += 1 } else { s.totals.requests_err += 1 }
                        // тут же обновляешь latency-агрегаты / окно RPS
                    }
                    Some(Event::InFlight{value}) => s.totals.in_flight = value,
                    Some(Event::RunFinished) | None => {
                        pb.finish_with_message("done");
                        break;
                    }
                }
            }
        }
    }
}

fn print_banner(version: &str) {
    println!();
    println!("{}", style("██╗     ████████╗").cyan().bold());
    println!("{}", style("██║     ╚══██╔══╝").cyan().bold());
    println!("{} {}{}", style("██║        ██║").cyan().bold(), style("Load Engine v").green().bold(), version);
    println!("{}", style("██║        ██║").cyan().bold());
    println!("{}", style("███████╗   ██║").cyan().bold());
    println!("{}", style("╚══════╝   ╚═╝").cyan().bold());
    println!();
}

pub fn validate(path: impl AsRef<Path>) ->anyhow::Result<()> {
    libprotocol::validate(path)?;
    Ok(())
}

pub fn export_schema(out_path: impl AsRef<Path>) -> anyhow::Result<()> {
    libprotocol::export_schema(out_path, Some("1".to_string()))
}

pub fn generate_scenario(out_path: impl AsRef<Path>) -> anyhow::Result<()> {
    libprotocol::generate_scenario(out_path, "1")
}
