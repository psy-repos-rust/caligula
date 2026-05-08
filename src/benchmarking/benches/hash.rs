use std::{fs::File, path::PathBuf};

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{
    benchmarking::{BenchContext, Benchmark, runner::BenchmarkParams},
    compression::CompressionFormat,
    hash::HashAlg,
    legacy_io::do_file_hashing,
};

/// File read and hash calculation benchmark.
#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
pub struct HashBenchParams {
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

impl BenchmarkParams for HashBenchParams {
    type Report = ();

    fn setup(&self, ctx: &BenchContext) -> Box<dyn Benchmark<Report = Self::Report>> {
        let this = self.clone();

        let file = File::open(&this.input).unwrap();
        let size = file.metadata().unwrap().len();
        ctx.set_progress_denominator(size);

        Box::new(move |ctx: &BenchContext| {
            do_file_hashing(file, this.compression, this.alg, |bs| {
                ctx.log_bytes_in(bs);
                ctx.log_progress(bs);
            })
            .unwrap();
            ctx.log_bytes_in(size);
            Box::new(|| ())
        })
    }
}
