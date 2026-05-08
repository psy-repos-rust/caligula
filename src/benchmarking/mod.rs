mod benches;
mod cli;
mod runner;

use cli::BenchSubcommands;

pub use self::{
    cli::BenchArgs,
    runner::{BenchContext, Benchmark},
};
use crate::benchmarking::runner::run_benchmark;

pub fn main(args: BenchArgs) {
    match args.command {
        BenchSubcommands::Run(args) => match args.type_ {
            benches::BenchType::Write(b) => run_benchmark(b),
            benches::BenchType::Hash(b) => run_benchmark(b),
            benches::BenchType::Verify(b) => run_benchmark(b),
        },
    }
}
