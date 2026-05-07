mod benches;
mod cli;
mod report;
mod result;
mod runner;

pub use self::cli::BenchArgs;
pub use self::runner::{BenchContext, Benchmark};
use cli::BenchSubcommands;

use crate::benchmarking::runner::run_benchmarks;

pub fn main(args: BenchArgs) {
    match args.command {
        BenchSubcommands::Run(args) => match args.type_ {
            benches::BenchType::Write(b) => run_benchmarks(b, args.runner_params),
            benches::BenchType::Hash(b) => run_benchmarks(b, args.runner_params),
            benches::BenchType::Verify(b) => run_benchmarks(b, args.runner_params),
        },
        BenchSubcommands::Report(args) => self::report::main(args),
    }
}
