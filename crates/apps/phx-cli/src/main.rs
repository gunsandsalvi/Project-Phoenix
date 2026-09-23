mod checks;
mod clock;
mod measure;
mod panic_hook;
mod run;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

/// The world's command line.
#[derive(Debug, Parser)]
#[command(name = "phx")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Assembles the world, settles it and runs it, with its live checks and report.
    Run(RunArgs),
    /// Measures what the budget reads.
    #[command(subcommand)]
    Measure(Measure),
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// The run's one seed.
    #[arg(long)]
    seed: u64,
    /// Days to run after settling.
    #[arg(long)]
    days: u16,
    /// Worker threads of the pool.
    #[arg(long)]
    workers: Option<usize>,
    /// Live checks to run: `all`, `none`, or a comma-separated list of identities.
    #[arg(long)]
    checks: String,
    /// Where to write the run's report.
    #[arg(long)]
    report: Option<PathBuf>,
    /// Traces reads and stream opens as they run.
    #[arg(long)]
    read_trace: bool,
    /// The world's data.
    #[arg(long)]
    data: PathBuf,
    /// The new game's setup.
    #[arg(long)]
    setup: PathBuf,
    /// The run's own directory, where the new game's countries are instantiated; never inside the repository's data.
    #[arg(long)]
    run_dir: PathBuf,
    /// The counters' ratchets.
    #[arg(long)]
    ratchets: PathBuf,
    /// The build's wall time, for the report.
    #[arg(long)]
    build_seconds: Option<u64>,
}

#[derive(Debug, Subcommand)]
enum Measure {
    /// The calendar's longest closed runs and its paydays after holidays.
    Calendar {
        #[arg(long)]
        data: PathBuf,
        #[arg(long)]
        setup: PathBuf,
        #[arg(long)]
        run_dir: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let outcome = match cli.command {
        Command::Run(args) => run::run(&args),
        Command::Measure(Measure::Calendar { data, setup, run_dir, out }) => {
            run::measure_calendar(&data, &setup, &run_dir, &out)
        }
    };
    match outcome {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
