use clap::Subcommand;
mod hash;
mod verify;
mod write;

pub use hash::HashBenchParams;
pub use verify::VerifyBench;
pub use write::WriteBench;

#[derive(Subcommand, Debug)]
pub enum BenchType {
    Write(write::WriteBench),
    Hash(hash::HashBenchParams),
    Verify(verify::VerifyBench),
}
