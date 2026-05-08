use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::benchmarking::{
    benches::{HashBenchParams, VerifyBench, WriteBench},
    runner::BenchmarkParams,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchRun<T> {
    pub common: CommonData,
    pub r#type: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommonData {
    pub date_ran: DateTime<Utc>,
    pub wall_time: Duration,
}

#[derive(Debug, Serialize, Deserialize, Clone, derive_more::From, derive_more::TryInto)]
pub enum AnyBenchType {
    Hash(BenchTypeData<HashBenchParams>),
    Write(BenchTypeData<WriteBench>),
    Verify(BenchTypeData<VerifyBench>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(bound = "P: Serialize + DeserializeOwned")]
pub struct BenchTypeData<P: BenchmarkParams> {
    pub params: P,
    pub result: P::Report,
}
