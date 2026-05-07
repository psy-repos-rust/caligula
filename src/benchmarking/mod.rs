mod benches;
mod cli;
mod report;
mod result;
mod runner;

pub use self::cli::BenchArgs;
pub use self::runner::{BenchContext, Benchmark};
use cli::BenchSubcommands;

use crate::benchmarking::runner::run_benchmark;

pub fn main(args: BenchArgs) {
    match args.command {
        BenchSubcommands::Run(args) => match args.type_ {
            benches::BenchTypes::Write(b) => run_benchmark(b),
            benches::BenchTypes::Hash(b) => run_benchmark(b),
            benches::BenchTypes::Verify(b) => run_benchmark(b),
        },
        BenchSubcommands::Report(args) => self::report::main(args),
    }
}
