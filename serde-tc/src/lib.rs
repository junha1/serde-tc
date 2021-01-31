use async_trait::async_trait;
pub use serde;
use std::collections::HashMap;

pub trait DispatchStringTuple {
    type Error: std::error::Error;
    fn dispatch(&self, arguments: &str) -> Result<String, Self::Error>;
}

pub trait DispatchStringDict {
    type Error: std::error::Error;
    type Poly;
    fn dispatch(&self, arguments: &HashMap<String, Self::Poly>) -> Result<String, Self::Error>;
}

#[async_trait]
pub trait DispatchStringTupleAsync {
    type Error: std::error::Error;
    async fn dispatch(&self, arguments: &str) -> Result<String, Self::Error>;
}

#[async_trait]
pub trait DispatchStringDictAsync {
    type Error: std::error::Error;
    type Poly;
    async fn dispatch(
        &self,
        arguments: &HashMap<String, Self::Poly>,
    ) -> Result<String, Self::Error>;
}
