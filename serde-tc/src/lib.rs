use async_trait::async_trait;
pub use serde;
pub use serde_tc_macro::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error<T: std::error::Error> {
    #[error("`{0}`")]
    MethodNotFound(String),
    #[error("`{0}`")]
    ArgumentNotFound(String),
    #[error("`{0}`")]
    Parse(T),
}

pub trait DispatchStringTuple {
    type Error: std::error::Error;
    fn dispatch(&self, method: &str, arguments: &str) -> Result<String, Error<Self::Error>>;
}

pub trait DispatchStringDict {
    type Error: std::error::Error;
    type Poly;
    fn dispatch(&self, method: &str, arguments: &str) -> Result<String, Error<Self::Error>>;
}

#[async_trait]
pub trait DispatchStringTupleAsync {
    type Error: std::error::Error;
    async fn dispatch(&self, method: &str, arguments: &str) -> Result<String, Error<Self::Error>>;
}

#[async_trait]
pub trait DispatchStringDictAsync {
    type Error: std::error::Error;
    type Poly;
    async fn dispatch(&self, method: &str, arguments: &str) -> Result<String, Error<Self::Error>>;
}
