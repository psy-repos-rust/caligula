use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread::Scope,
    time::{Duration, Instant},
};

use bytesize::ByteSize;
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    benchmarking::result::{BenchRun, BenchRunType, BenchTypeData},
    ui::ByteSpeed,
};

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
    #[arg(short, long)]
    pub output_file: Option<PathBuf>,

    /// If provided, the JSON results will be formatted with pretty indentation.
    #[arg(long)]
    pub output_pretty: bool,
}

pub fn run_benchmarks<B: BenchmarkParams>(bench_params: B, runner_params: BenchRunnerParams)
where
    BenchRunType: From<BenchTypeData<B>>,
{
    let mut output: Box<dyn Write> = match runner_params.output_file {
        Some(f) => Box::new(File::create(f).expect("Failed to open output for writing")),
        None => Box::new(std::io::stdout()),
    };

    let cooldown = Duration::from_secs(runner_params.cooldown_secs.into());
    for i in 1..=runner_params.count {
        let ctx = BenchContext::default();
        let bench = bench_params.setup(&ctx);

        let date_ran = Utc::now();
        let count = runner_params.count;
        let ctxref = &ctx;
        let (wall_time, result) = std::thread::scope(move |s| run_once(i, count, bench, ctxref, s));

        let run_result = BenchRun {
            date_ran,
            wall_time,
            r#type: BenchRunType::from(BenchTypeData {
                params: bench_params.clone(),
                result,
            }),
        };

        (|| {
            if runner_params.output_pretty {
                serde_json::to_writer_pretty(&mut output, &run_result)?;
            } else {
                serde_json::to_writer(&mut output, &run_result)?;
            }
            writeln!(output)?;
            output.flush()?;
            Ok::<_, std::io::Error>(())
        })()
        .expect("Failed to write bench result to output!");

        if !cooldown.is_zero() && i != count {
            eprintln!("Pausing for {cooldown:?}");
            std::thread::sleep(cooldown);
        }
    }
}

fn run_once<'scope, 'env, R>(
    i: u32,
    total: u32,
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
        .with_message("Bench")
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
    let (wall_time, report) = handle.join().unwrap();
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

    (wall_time, report)
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

fn make_progress_style(i: u32, total: u32, is_done: bool) -> ProgressStyle {
    use std::fmt::Write as _;

    const IN_PROGRESS: &str = "[{elapsed_precise}] {wide_bar:.yellow} {percent:>3}%";
    const DONE: &str = "[{elapsed_precise}] {wide_bar:.green} {percent:>3}%";

    let mut template = String::from("{msg} ");

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
