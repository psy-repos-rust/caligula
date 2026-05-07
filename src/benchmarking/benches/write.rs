use std::{fs::File, path::PathBuf};

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{
    benchmarking::{BenchContext, Benchmark},
    compression::CompressionFormat,
    legacy_io::WriteOp,
};

/// Disk write benchmark.
#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
pub struct WriteBench {
    /// Input file to write.
    #[arg(short, long, display_order = 0)]
    pub image: PathBuf,

    /// Disk to write to. THIS WILL BE ERASED WITHOUT CONFIRMATION!!!
    #[arg(short = 'o', long, display_order = 1)]
    // needs display_order = 1 or else it will go above image
    pub disk: PathBuf,

    /// What compression format the input file is in.
    #[arg(short = 'z', long, default_value = "none")]
    pub compression: CompressionFormat,

    #[arg(long, default_value = "1048576")]
    pub file_read_buf_size: usize,

    #[arg(long, default_value = "1048576")]
    pub disk_write_buf_size: usize,

    #[arg(long, default_value = "4096")]
    pub disk_block_size: usize,
}

impl Benchmark for WriteBench {
    fn run(self: Self, ctx: &BenchContext) {
        WriteOp {
            file: File::open(self.image).expect("failed to open image"),
            disk: File::open(self.disk).expect("failed to open disk"),
            cf: self.compression,
            buf_size: self.disk_write_buf_size,
            disk_block_size: self.disk_block_size,
            checkpoint_period: 32,
            file_read_buf_size: self.file_read_buf_size,
        }
        .execute(|e| match e {
            crate::herder_api::write_verify::WriteVerifyEvent::TotalBytes { src, dest } => {
                ctx.log_bytes_in(src);
                ctx.log_bytes_out(dest);
            }
            _ => (),
        })
        .expect("operation failed");
    }

    fn progress_denominator(&self) -> u64 {
        std::fs::metadata(&self.image).unwrap().len()
    }
}
