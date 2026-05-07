use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{
    benchmarking::{BenchContext, Benchmark},
    compression::CompressionFormat,
};

/// Disk verification benchmark.
#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
pub struct VerifyBench {
    /// Input file to verify against.
    #[arg(short, long, display_order = 0)]
    pub input: PathBuf,

    /// Disk to verify.
    #[arg(short = 'o', long, display_order = 1)]
    // needs display_order = 1 or else it will go above image
    pub disk: PathBuf,

    /// What compression format the input file is in.
    #[arg(short = 'z', long, default_value = "identity")]
    pub compression: CompressionFormat,
}

impl Benchmark for VerifyBench {
    fn run(self: Self, _ctx: &BenchContext) {
        todo!()
    }

    fn progress_denominator(&self) -> u64 {
        std::fs::metadata(&self.input).unwrap().len()
    }
}
