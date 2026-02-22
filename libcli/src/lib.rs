use clap::{Parser, Subcommand, ValueEnum};
use std::path::Path;

/// A fictional versioning CLI
#[derive(Debug, Parser)] // requires `derive` feature
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

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum ColorWhen {
    Always,
    Auto,
    Never,
}

impl std::fmt::Display for ColorWhen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
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
            Ok(libruntime::dry_run(scenario, seed, iterations, is_simulated).await)
        },
        Commands::RunMock { scenario} => {
            Ok(libruntime::run(scenario, Some(true)).await)
        },
        Commands::Run { scenario} => {
            Ok(libruntime::run(scenario, Some(false)).await)
        },

    }
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
