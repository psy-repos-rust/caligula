use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread::Scope,
    time::{Duration, Instant},
};

use bytesize::ByteSize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Serialize, de::DeserializeOwned};

use crate::ui::ByteSpeed;

const REFRESH_PERIOD: Duration = Duration::from_millis(250);

#[derive(clap::Parser, Debug)]
pub struct BenchRunnerParams {
    /// How many times to run the requested benchmark.
    #[arg(short = 'n', long, default_value = "1")]
    pub count: u32,

    /// Cooldown period to wait between repetitions.
    #[arg(short = 'T', long, default_value = "0")]
    pub cooldown_secs: u32,

    /// File to write JSON results to. If not specified, writes to stdout.
    #[arg(short, long, default_value = "0")]
    pub output_file: Option<PathBuf>,
}

pub fn run_benchmarks(b: impl BenchmarkParams, params: BenchRunnerParams) {
    let cooldown = Duration::from_secs(params.cooldown_secs.into());
    for i in 1..=params.count {
        let ctx = BenchContext::default();
        let b = b.setup(&ctx);

        let count = params.count;
        let ctxref = &ctx;
        std::thread::scope(move |s| {
            run_once(i, count, b, ctxref, s);
        });

        if !cooldown.is_zero() && i != count {
            eprintln!("Pausing for {cooldown:?}");
            std::thread::sleep(cooldown);
        }
    }
}

fn run_once<'scope, 'env>(
    i: u32,
    total: u32,
    b: Box<dyn Benchmark>,
    ctx: &'scope BenchContext,
    s: &'scope Scope<'scope, 'env>,
) {
    // spawn the thread
    let handle = s.spawn(move || {
        let start = Instant::now();
        b.run(ctx);
        start.elapsed()
    });

    // render a progress bar!
    let len = 80;
    let bar = ProgressBar::new(len)
        .with_message("Running benchmark")
        .with_style(make_progress_style(i, total, false));

    // omg so pretty
    while !handle.is_finished() {
        std::thread::sleep(REFRESH_PERIOD);
        let denominator = ctx.denominator.load(Ordering::Relaxed);
        let progress = ctx.progress.load(Ordering::Relaxed);
        bar.set_position((progress as f64 * len as f64 / denominator as f64) as u64);
    }

    // set to 100% progress
    bar.set_style(make_progress_style(i, total, true));
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
}

/// Interface available for benchmarks to log data to.
#[derive(Default)]
pub struct BenchContext {
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
    progress: AtomicU64,
    denominator: AtomicU64,
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

    pub fn set_progress_denominator(&self, denominator: u64) {
        self.denominator.store(denominator, Ordering::Relaxed);
    }
}

/// Canned arguments for creating benchmarks.
pub trait BenchmarkParams: Sync + Serialize + DeserializeOwned + 'static {
    /// Prepare a benchmark to be executed.
    fn setup(&self, ctx: &BenchContext) -> Box<dyn Benchmark>;
}

/// A benchmark that has been fully set up, and is ready to be executed.
pub trait Benchmark: Send + 'static {
    /// Execute the benchmark.
    fn run(self: Box<Self>, ctx: &BenchContext);
}

impl<F> Benchmark for F
where
    F: FnOnce(&BenchContext) + Send + 'static,
{
    fn run(self: Box<Self>, ctx: &BenchContext) {
        (self)(ctx)
    }
}

fn make_progress_style(i: u32, total: u32, is_done: bool) -> ProgressStyle {
    use std::fmt::Write as _;

    const IN_PROGRESS: &str = "[{elapsed_precise}] {msg:>10} {wide_bar:.yellow} {percent:>3}%";
    const DONE: &str = "[{elapsed_precise}] {msg:>10} {wide_bar:.green} {percent:>3}%";

    let mut template = String::new();

    // left pad
    let max_len = total.to_string().len();
    let i = i.to_string();
    for _ in 0..(max_len - i.len()) {
        template.push(' ');
    }

    write!(&mut template, "{i}/{total}: ").unwrap();

    match is_done {
        true => template.push_str(DONE),
        false => template.push_str(IN_PROGRESS),
    }

    ProgressStyle::with_template(&template).unwrap()
}
