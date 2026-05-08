use clap::Subcommand;
mod hash;
mod verify;
mod write;

#[derive(Subcommand, Debug)]
pub enum BenchType {
    Write(write::WriteBench),
    Hash(hash::HashBenchParams),
    Verify(verify::VerifyBench),
}
