use std::path::PathBuf;

use clap::Parser;

use crate::{benchmarking::benches::Benchmark, hash::HashAlg};

/// File read and hash calculation benchmark.
#[derive(Parser, Debug)]
pub struct HashBench {
    /// Input image to hash.
    #[arg(display_order = 0)]
    pub input: PathBuf,

    /// What hash algorithm to run over the input.
    #[arg(short = 'a', long)]
    pub alg: HashAlg,
}

impl Benchmark for HashBench {}
