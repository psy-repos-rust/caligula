use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use bytesize::ByteSize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Serialize, de::DeserializeOwned};

use crate::ui::ByteSpeed;

const REFRESH_PERIOD: Duration = Duration::from_millis(250);

pub fn run_benchmark(b: impl Benchmark) {
    let ctx = BenchContext::default();
    let denominator = b.progress_denominator();
    std::thread::scope(|s| {
        // set up the benchmark
        let b = b.clone();
        let ctx = &ctx;

        // run it in a scoped thread
        let handle = s.spawn(move || {
            let start = Instant::now();
            b.run(ctx);
            start.elapsed()
        });

        // render a progress bar!
        const IN_PROGRESS: &str = "[{elapsed_precise}] {msg:>10} {wide_bar:.yellow} {percent:>3}%";
        const DONE: &str = "[{elapsed_precise}] {msg:>10} {wide_bar:.green} {percent:>3}%";
        let len = 80;
        let bar = ProgressBar::new(len)
            .with_message("Running benchmark")
            .with_style(ProgressStyle::with_template(IN_PROGRESS).unwrap());

        // omg so pretty
        while !handle.is_finished() {
            std::thread::sleep(REFRESH_PERIOD);
            let progress = ctx.progress.load(Ordering::Relaxed);
            bar.set_position((progress as f32 * len as f32 / denominator as f32) as u64);
        }

        // set to 100% progress
        bar.set_style(ProgressStyle::with_template(DONE).unwrap());
        bar.set_position(len);

        // print the report
        let wall_time = handle.join().unwrap();
        let bytes_in = ctx.bytes_in.load(Ordering::Relaxed);
        let bytes_out = ctx.bytes_out.load(Ordering::Relaxed);
        eprintln!("Time elapsed: {wall_time:?}");
        eprintln!("Bytes in:     {}", ByteSize::b(bytes_in));
        eprintln!("Bytes out:    {}", ByteSize::b(bytes_out));
        eprintln!(
            "Input rate:   {}",
            ByteSpeed(bytes_in as f64 / wall_time.as_secs_f64())
        );
        eprintln!(
            "Output rate:  {}",
            ByteSpeed(bytes_out as f64 / wall_time.as_secs_f64())
        );
    });
}

#[derive(Default)]
pub struct BenchContext {
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
    progress: AtomicU64,
}

impl BenchContext {
    pub fn log_bytes_in(&self, bytes_in: u64) {
        self.bytes_in.store(bytes_in, Ordering::Relaxed);
    }

    pub fn log_bytes_out(&self, bytes_out: u64) {
        self.bytes_out.store(bytes_out, Ordering::Relaxed);
    }

    pub fn log_progress(&self, progress: u64) {
        self.progress.store(progress, Ordering::Relaxed);
    }
}

pub trait Benchmark: Clone + Sized + Send + Serialize + DeserializeOwned + 'static {
    fn progress_denominator(&self) -> u64;
    fn run(self: Self, ctx: &BenchContext);
}
