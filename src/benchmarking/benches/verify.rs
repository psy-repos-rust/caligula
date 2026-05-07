use std::path::PathBuf;

use clap::Parser;

use crate::{benchmarking::benches::Benchmark, compression::CompressionArg};

/// Disk verification benchmark.
#[derive(Parser, Debug)]
pub struct VerifyBench {
    /// Input file to verify against.
    #[arg(short, long, display_order = 0)]
    pub input: PathBuf,

    /// Disk to verify.
    #[arg(short = 'o', long, display_order = 1)] // needs display_order = 1 or else it will go above image
    pub disk: PathBuf,

    /// What compression format the input file is in.
    #[arg(short = 'z', long, default_value = "none")]
    pub compression: CompressionArg,
}

impl Benchmark for VerifyBench {}
