use std::{
    sync::atomic::{AtomicU64, Ordering},
    thread::Scope,
    time::{Duration, Instant},
};

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use serde::{Serialize, de::DeserializeOwned};

const REFRESH_PERIOD: Duration = Duration::from_millis(250);

pub fn run_benchmark<B: BenchmarkParams>(bench_params: B) {
    let ctx = BenchContext::default();
    let bench = bench_params.setup(&ctx);

    let ctxref = &ctx;
    std::thread::scope(move |s| run_once(bench, ctxref, s));
}

fn run_once<'scope, 'env, R>(
    b: Box<dyn Benchmark<Report = R>>,
    ctx: &'scope BenchContext,
    s: &'scope Scope<'scope, 'env>,
) -> (Duration, R)
where
    R: Serialize + DeserializeOwned + Send + 'static,
{
    // spawn the thread
    let handle = s.spawn(move || {
        let start = Instant::now();
        let report = b.run(ctx);
        let elapsed = start.elapsed();
        (elapsed, report())
    });

    // render a progress bar!
    let len = 80;
    let bar = ProgressBar::new(len)
        .with_message("Benching")
        .with_style(make_progress_style(false));
    bar.set_draw_target(ProgressDrawTarget::stderr());

    // omg so pretty
    while !handle.is_finished() {
        std::thread::sleep(REFRESH_PERIOD);
        let denominator = ctx.denominator.load(Ordering::Relaxed);
        let progress = ctx.progress.load(Ordering::Relaxed);
        if denominator != 0 {
            bar.set_position((progress as f64 * len as f64 / denominator as f64) as u64);
        }
    }

    // set to 100% progress
    bar.set_style(make_progress_style(true));
    bar.set_position(len);
    bar.finish_with_message("Done!");

    // print the report
    let (wall_time, report) = handle.join().unwrap();

    (wall_time, report)
}

/// Interface available for benchmarks to log data to.
#[derive(Default)]
pub struct BenchContext {
    progress: AtomicU64,
    denominator: AtomicU64,
}

impl BenchContext {
    /// Set the denominator to use for progress tracking.
    pub fn set_progress_denominator(&self, denominator: u64) {
        self.denominator.store(denominator, Ordering::Relaxed);
    }

    /// Log progress. This can be any arbitrary unit, but in order for it to
    /// take effect, [`Self::set_progress_denominator()`] must have been
    /// called with a non-zero value.
    pub fn log_progress(&self, progress: u64) {
        self.progress.store(progress, Ordering::Relaxed);
    }
}

/// Canned arguments for creating benchmarks.
pub trait BenchmarkParams: Clone + Sync + Serialize + DeserializeOwned + 'static {
    /// Additional data to report from this benchmark.
    type Report: Serialize + DeserializeOwned + Send + 'static;

    /// Prepare a benchmark to be executed.
    fn setup(&self, ctx: &BenchContext) -> Box<dyn Benchmark<Report = Self::Report>>;
}

/// A benchmark that has been fully set up, and is ready to be executed.
pub trait Benchmark: Send + 'static {
    /// Additional data to report from this benchmark.
    type Report: Serialize + DeserializeOwned + Send + 'static;

    /// Execute the benchmark.
    fn run(self: Box<Self>, ctx: &BenchContext) -> Box<dyn FnOnce() -> Self::Report>;
}

impl<F, R, RF> Benchmark for F
where
    F: FnOnce(&BenchContext) -> RF + Send + 'static,
    R: Serialize + DeserializeOwned + Send + 'static,
    RF: FnOnce() -> R + 'static,
{
    type Report = R;

    fn run(self: Box<Self>, ctx: &BenchContext) -> Box<dyn FnOnce() -> Self::Report> {
        Box::new((self)(ctx))
    }
}

fn make_progress_style(is_done: bool) -> ProgressStyle {
    const IN_PROGRESS: &str = "[{elapsed_precise}] {wide_bar:.yellow} {percent:>3}%";
    const DONE: &str = "[{elapsed_precise}] {wide_bar:.green} {percent:>3}%";

    let template = match is_done {
        true => DONE,
        false => IN_PROGRESS,
    };

    ProgressStyle::with_template(template).unwrap()
}
