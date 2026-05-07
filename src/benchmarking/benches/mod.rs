use clap::Subcommand;
mod hash;
mod verify;
mod write;

#[derive(Subcommand, Debug)]
pub enum BenchTypes {
    Write(write::WriteBench),
    Hash(hash::HashBench),
    Verify(verify::VerifyBench),
}
