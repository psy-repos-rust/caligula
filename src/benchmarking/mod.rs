mod benches;
mod cli;
mod report;
mod result;

pub use cli::BenchArgs;
use cli::BenchSubcommands;

use crate::benchmarking::benches::Benchmark;

pub fn main(args: BenchArgs) {
    match args.command {
        BenchSubcommands::Run(args) => match args.type_ {
            benches::BenchTypes::Write(b) => b.run(),
            benches::BenchTypes::Hash(b) => b.run(),
            benches::BenchTypes::Verify(b) => b.run(),
        },
        BenchSubcommands::Report(args) => self::report::main(args),
    }
}
