use serde::{Deserialize, Serialize};

/// A UNIX timestamp in microsecs.
pub type Timestamp = i64;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct StrategyId(String);

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct UserId(String);

// TODO: define market data types here and make `kex-exchange` import this module.
pub mod kex {}
