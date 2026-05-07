use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{
    benchmarking::{BenchContext, Benchmark},
    compression::CompressionFormat,
};

/// Disk write benchmark.
#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
pub struct WriteBench {
    /// Input file to write against.
    #[arg(short, long, display_order = 0)]
    pub input: PathBuf,

    /// Disk to write to. THIS WILL BE ERASED WITHOUT CONFIRMATION!!!
    #[arg(short = 'o', long, display_order = 1)]
    // needs display_order = 1 or else it will go above image
    pub disk: PathBuf,

    /// What compression format the input file is in.
    #[arg(short = 'z', long, default_value = "none")]
    pub compression: CompressionFormat,
}

impl Benchmark for WriteBench {
    fn run(self: Self, _ctx: &BenchContext) {
        todo!()
    }
}
