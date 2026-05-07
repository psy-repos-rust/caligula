use std::{fs::File, path::PathBuf};

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{
    benchmarking::{BenchContext, Benchmark},
    compression::CompressionFormat,
    hash::HashAlg,
    legacy_io::do_file_hashing,
};

/// File read and hash calculation benchmark.
#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
pub struct HashBench {
    /// Input image to hash.
    #[arg(display_order = 0)]
    pub input: PathBuf,

    /// What hash algorithm to run over the input.
    #[arg(short = 'a', long)]
    pub alg: HashAlg,

    /// What compression format the input file is in.
    #[arg(short = 'z', long, default_value = "identity")]
    pub compression: CompressionFormat,
}

impl Benchmark for HashBench {
    fn run(self: Self, _ctx: &BenchContext) {
        do_file_hashing(
            File::open(self.input).unwrap(),
            self.compression,
            self.alg,
            |_| {},
        )
        .unwrap();
    }
}
