use async_trait::async_trait;
use hdk_common::crypto::PublicKey;
use network::*;
use serde_json::Value;
use serde_tc::*;
use std::sync::Arc;
use tokio::sync::RwLock;

/// This macro generates 2 new useful entities:
/// `trait CalculatorFallible` and `struct CalculatorStub`.
/// `CalculatorFallible` is another trait that imposes `Result<_, anyhow::Error>` for all methods.
/// `CalculatorStub` is a struct that implements `CalculatorFallible` using the given abstract client `Box<dyn StubCall>`.
/// `serde-tc` provides trivial impl. of `StubCall` (`HttpClient`) which will be used for most of the cases.
#[serde_tc_full]
trait Calculator: Send + Sync {
    async fn add(&self, value: i64);
    async fn is_bigger_than(&self, value: i64) -> bool;
    async fn get(&self) -> i64;
    async fn reset(&self);
}

struct SimpleCalculator {
    value: RwLock<i64>,
}

#[async_trait]
impl CalculatorServer for SimpleCalculator {
    async fn add(&self, _caller_key: PublicKey, value: i64) {
        let mut value_ = self.value.write().await;
        *value_ += value;
    }

    async fn is_bigger_than(&self, _caller_key: PublicKey, value: i64) -> bool {
        let value_ = self.value.read().await;
        *value_ > value
    }

    async fn get(&self, _caller_key: PublicKey) -> i64 {
        let value_ = self.value.read().await;
        *value_
    }

    async fn reset(&self, _caller_key: PublicKey) {
        let mut value_ = self.value.write().await;
        *value_ = 0;
    }
}

/// Server side code;
/// all you need to do is just providing a impl. of `Calculator` (`SimpleCalculator`) to the `run_server()`,
/// after converting the imp. to an abstract HTTP-serving object using `create_http_object()`.
async fn server() {
    run_server(
        14123,
        [(
            "x".to_owned(),
            create_http_object(Arc::new(SimpleCalculator {
                value: RwLock::new(0),
            }) as Arc<dyn CalculatorServer>),
        )]
        .iter()
        .cloned()
        .collect(),
    )
    .await;
}

/// Client side case 1;
/// You somehow directly have the code-level definition of `trait Calculator` (expaneded by `![serde_tc_macro]`).
/// Then you can use the auto-generated `CalculatorStub` which implements `CalculatorFallible`.
async fn client_trait_aware() {
    let client = CalculatorStub::new(Box::new(HttpClient::new("localhost:14123/x".to_owned())));
    client.reset().await.unwrap();
    client.add(1).await.unwrap();
    assert_eq!(client.get().await.unwrap(), 1);
    client.add(2).await.unwrap();
    assert_eq!(client.get().await.unwrap(), 3);
    assert!(!client.is_bigger_than(3).await.unwrap());
    assert!(client.is_bigger_than(2).await.unwrap());
    client.reset().await.unwrap();
    assert_eq!(client.get().await.unwrap(), 0);
}

#[tokio::main]
async fn main() {
    color_eyre::install().unwrap();
    tokio::task::spawn(server());
    client_trait_aware().await;
}
