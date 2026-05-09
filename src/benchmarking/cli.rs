use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::benchmarking::benches::BenchType;

const BENCH_ABOUT: &str = "Caligula benchmarking subsystem.

WARNING: THESE COMMANDS ARE EXTREMELY DANGEROUS! THIS IS A SUBSYSTEM MEANT FOR CALIGULA \
                           DEVELOPERS, RATHER THAN END USERS! COMMAND STABILITY IS NOT \
                           GUARANTEED, AND IF YOU USE THIS, DO NOT EXPECT TO RECEIVE ANY HELP \
                           WHATSOEVER! YOU HAVE BEEN WARNED!";

#[derive(Parser, Debug)]
#[clap(about = BENCH_ABOUT)]
pub struct BenchArgs {
    #[command(subcommand)]
    pub command: BenchSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum BenchSubcommands {
    Run(RunBenchArgs),
}

/// Generate a report from one or multiple benchmark runs.
#[derive(Parser, Debug)]
pub struct ReportBenchArgs {
    /// Files to read from. If not provided, reads from stdin.
    pub result_files: Vec<PathBuf>,

    /// Tags to consider as the "base" benchmark, or empty to not work in
    /// comparison mode. Any runs not having this tag will be considered the
    /// "comparison."
    #[arg(short, long, value_delimiter = ',')]
    pub base: Vec<String>,
}

/// Run a benchmark.
///
/// All benchmarks assume you have adequate permissions to read and write the
/// files you pass in. No auto-escalation is done.
#[derive(Parser, Debug)]
pub struct RunBenchArgs {
    /// Which benchmark to run.
    #[command(name = "type", subcommand)]
    pub type_: BenchType,
}
