use std::{
    fs::File,
    io::{Read, Write},
};

use itertools::Itertools as _;

use crate::benchmarking::{cli::ReportBenchArgs, result::BenchRun};

pub fn main(args: ReportBenchArgs) {
    let inputs: Vec<Box<dyn Read>> = match args.result_files.len() {
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

    let output: Box<dyn Write> = match args.output_file {
        Some(f) => Box::new(File::create(f).expect("Failed to open output for writing")),
        None => Box::new(std::io::stdout()),
    };

    let benches = read_benches(inputs).expect("error reading benchmarks");
    write_report(output, benches).expect("error generating report");
}

/// Read all benchmarks contained in the list of [`Read`]s provided.
fn read_benches(_r: Vec<impl Read>) -> std::io::Result<Vec<BenchRun>> {
    todo!()
}

fn write_report(_w: impl Write, _benches: Vec<BenchRun>) -> std::io::Result<()> {
    todo!()
}
