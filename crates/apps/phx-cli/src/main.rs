mod budget;
mod checks;
mod clock;
mod fin;
mod inject;
mod panic_hook;
mod run;
mod trace;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

/// The bench's build counts every allocation, so its trace tells each span's.
#[cfg(feature = "bench")]
#[global_allocator]
static ALLOC: phx_exec::alloc::CountingAlloc = phx_exec::alloc::CountingAlloc;

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
    /// Injects a family's discrepancy into a save loaded apart, audits it and discards it.
    Inject(InjectArgs),
    /// Measures the finished world's volumes: each base filled at the design point and run through its kernels.
    Fin(FinArgs),
}

#[derive(Debug, Args)]
pub struct FinArgs {
    /// The design point's figure set.
    #[arg(long)]
    design: PathBuf,
    /// The budget, whose `fin.` ratchets the measures are held to.
    #[arg(long)]
    budget: PathBuf,
    /// The bases to fill and measure, comma-separated, or `all`.
    #[arg(long, default_value = "all")]
    bases: String,
    /// The day types to run, comma-separated among `B`, `NB`, `H` and `BC`, or `turn` for every one.
    #[arg(long, default_value = "turn")]
    days: String,
    /// Where to write the report.
    #[arg(long)]
    report: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct InjectArgs {
    /// The save's directory.
    #[arg(long)]
    from: PathBuf,
    /// The one family to inject; every family, each into its own load, when absent.
    #[arg(long)]
    family: Option<String>,
    /// The world's data the save was written over.
    #[arg(long)]
    data: PathBuf,
    /// The new game's setup.
    #[arg(long)]
    setup: PathBuf,
    /// The load's own directory, where the countries are instantiated.
    #[arg(long)]
    run_dir: PathBuf,
    /// Where to write the injections' report.
    #[arg(long)]
    report: Option<PathBuf>,
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    let outcome = match cli.command {
        Command::Run(args) => run::run(&args),
        Command::Fin(a) => fin::run(&a),
        Command::Inject(a) => {
            inject::run(&a.from, &a.data, &a.setup, &a.run_dir, a.family.as_deref(), a.report.as_deref())
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
