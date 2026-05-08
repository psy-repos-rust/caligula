use std::{
    fmt::Debug,
    fs::File,
    io::{Read, Write},
};

use itertools::Itertools as _;

use crate::benchmarking::{
    benches::{HashBenchParams, VerifyBench, WriteBench},
    cli::ReportBenchArgs,
    result::{AnyBenchType, BenchRun, BenchTypeData},
    runner::BenchmarkParams,
};

pub fn main(args: ReportBenchArgs) {
    let rs: Vec<Box<dyn Read>> = match args.result_files.len() {
        0 => {
            vec![Box::new(std::io::stdin())]
        }
        _ => args
            .result_files
            .into_iter()
            .map(File::open)
            .map_ok(|f| -> Box<dyn Read> { Box::new(f) })
            .collect::<std::io::Result<Vec<Box<dyn Read>>>>()
            .expect("Failed to open inputs for reading"),
    };

    let w: Box<dyn Write> = match args.output_file {
        Some(f) => Box::new(File::create(f).expect("Failed to open output for writing")),
        None => Box::new(std::io::stdout()),
    };

    let mut benches = read_benches(rs).peekable();
    let first = benches.peek().expect("got empty list of benchmarks!");

    match first.r#type {
        AnyBenchType::Hash(_) => write_report::<HashBenchParams>(w, downcast_benches(benches)),
        AnyBenchType::Write(_) => write_report::<WriteBench>(w, downcast_benches(benches)),
        AnyBenchType::Verify(_) => write_report::<VerifyBench>(w, downcast_benches(benches)),
    }
    .expect("Failed to write report!");
}

/// Read all benchmarks contained in the list of [`Read`]s provided.
fn read_benches(r: Vec<impl Read>) -> impl Iterator<Item = BenchRun<AnyBenchType>> {
    r.into_iter()
        .flat_map(|r| serde_json::Deserializer::from_reader(r).into_iter())
        .map(|x| x.expect("Failed to deserialize"))
}

fn downcast_benches<T, E>(
    benches: impl IntoIterator<Item = BenchRun<AnyBenchType>>,
) -> impl IntoIterator<Item = BenchRun<T>>
where
    T: TryFrom<AnyBenchType, Error = E>,
    E: Debug,
{
    benches.into_iter().map(|x| BenchRun {
        common: x.common,
        r#type: T::try_from(x.r#type).expect("Not all the same type!"),
    })
}

fn write_report<B: BenchmarkParams>(
    w: impl Write,
    benches: impl IntoIterator<Item = BenchRun<BenchTypeData<B>>>,
) -> std::io::Result<()> {
    let benches = benches.into_iter().collect_vec();
    todo!()
}
