use async_trait::async_trait;
pub use serde;
pub use serde_tc_macro::{serde_tc_str, serde_tc_str_debug};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DictError<T: std::error::Error> {
    #[error("`{0}`")]
    ArgumentNotFound(String),
    #[error("`{0}`")]
    Parse(T),
}

pub trait DispatchStringTuple {
    type Error: std::error::Error;
    fn dispatch(&self, method: &str, arguments: &str) -> Result<String, Self::Error>;
}

pub trait DispatchStringDict {
    type Error: std::error::Error;
    type Poly;
    fn dispatch(
        &self,
        method: &str,
        arguments: &HashMap<String, Self::Poly>,
    ) -> Result<String, DictError<Self::Error>>;
}

#[async_trait]
pub trait DispatchStringTupleAsync {
    type Error: std::error::Error;
    async fn dispatch(&self, method: &str, arguments: &str) -> Result<String, Self::Error>;
}

#[async_trait]
pub trait DispatchStringDictAsync {
    type Error: std::error::Error;
    type Poly;
    async fn dispatch(
        &self,
        method: &str,
        arguments: &HashMap<String, Self::Poly>,
    ) -> Result<String, DictError<Self::Error>>;
}
