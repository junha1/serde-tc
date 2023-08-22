/*!
`serde-tc` is a library for de/serializing method invocations for trait objects.

`serde-tc-macro` provides a macro for generating various code for a particular trait defiiniation.
1. A dispatcher; it takes the method name and the arguemnts (an opaque string) and invokes the method on the object.
2. A encoder; it defines a copy of the methods of the trait. Instead of the original return types,
the newly defined methods return encoded strings that can be directly used by the dispatcher.

`serde-tc` also provides a convenient module `network`,
which automatically builds a HTTP server using the given trait objects
to serve as a RPC server. The module also provides a `stub` implementation,
which can be used as the HTTP client when the server is built with the same trait.

It also provides an authentication layer out of the box, where the client signs the request
and the server verifies the signature. A reserved parameter name `caller_key` represents
the public key of the client, which is automatically verified and injected by the authentication layer.

Please refer to `tests/integration_tests.rs` or `examples/calculator.rs` for the actual usage.
*/

pub mod network;

use async_trait::async_trait;
pub use serde;
pub use serde_tc_macro::*;
use std::sync::Arc;
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

#[async_trait]
impl<T> DispatchStringDictAsync for Arc<T>
where
    T: DispatchStringDictAsync + Send + Sync + 'static + ?Sized,
{
    type Error = T::Error;
    type Poly = T::Poly;
    async fn dispatch(&self, method: &str, arguments: &str) -> Result<String, Error<Self::Error>> {
        (self.as_ref() as &T).dispatch(method, arguments).await
    }
}

#[async_trait]
impl<T> DispatchStringTupleAsync for Arc<T>
where
    T: DispatchStringTupleAsync + Send + Sync + 'static + ?Sized,
{
    type Error = T::Error;
    async fn dispatch(&self, method: &str, arguments: &str) -> Result<String, Error<Self::Error>> {
        (self.as_ref() as &T).dispatch(method, arguments).await
    }
}

#[async_trait]
pub trait StubCall: Send + Sync {
    type Error;

    async fn call(&self, method: &'static str, params: String) -> Result<String, Self::Error>;
}
