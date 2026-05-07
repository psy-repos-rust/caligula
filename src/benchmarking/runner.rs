use std::time::Instant;

use serde::{Serialize, de::DeserializeOwned};

pub fn run_benchmark(b: impl Benchmark) {
    let ctx = BenchContext {};
    std::thread::scope(|s| {
        let b = b.clone();
        let ctx = &ctx;
        let wall_time = s
            .spawn(move || {
                let start = Instant::now();
                b.run(ctx);
                start.elapsed()
            })
            .join()
            .unwrap();
        eprintln!("Result: {wall_time:?}")
    });
}

pub struct BenchContext {}

impl BenchContext {}

pub trait Benchmark: Clone + Sized + Send + Serialize + DeserializeOwned + 'static {
    fn run(self: Self, ctx: &BenchContext);
}
