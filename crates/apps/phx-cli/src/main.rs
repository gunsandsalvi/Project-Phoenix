mod budget;
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
    /// Instead, days to run from day zero in all, settling cut short: an ordinary step's build run.
    #[arg(long)]
    total_days: Option<u16>,
    /// Worker threads of the pool.
    #[arg(long)]
    workers: Option<usize>,
    /// Live checks to run: `all`, `none`, or a comma-separated list of identities.
    #[arg(long)]
    checks: String,
    /// Where to write the run's report.
    #[arg(long)]
    report: Option<PathBuf>,
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
    /// The budget's ratchets: turn times, memory a person, cores busy and page faults, held apart from the counters'.
    #[arg(long)]
    budget: Option<PathBuf>,
    /// The build's wall time, for the report.
    #[arg(long)]
    build_seconds: Option<u64>,
    /// The persons the world holds in place of the data's, of the setup's population.
    #[arg(long)]
    persons: Option<u64>,
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
    /// The budget as measured: the phone's turns, sub-steps, memory and unit costs, and the full-load bench.
    Budget {
        /// The build run's report of the commit the phone ran.
        #[arg(long)]
        build_run: PathBuf,
        /// The phone's device report.
        #[arg(long)]
        device: PathBuf,
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
        Command::Measure(Measure::Budget { build_run, device, out }) => run::measure_budget(&build_run, &device, &out),
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
