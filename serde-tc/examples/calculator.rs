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
    async fn add(&self, instance_key: PublicKey, value: i64) -> Result<(), eyre::Error>;
    async fn is_bigger_than(
        &self,
        instance_key: PublicKey,
        value: i64,
    ) -> Result<bool, eyre::Error>;
    async fn get(&self, instance_key: PublicKey) -> Result<i64, eyre::Error>;
    async fn reset(&self, instance_key: PublicKey) -> Result<(), eyre::Error>;
}

struct SimpleCalculator {
    value: RwLock<i64>,
}

#[async_trait]
impl Calculator for SimpleCalculator {
    async fn add(&self, instance_key: PublicKey, value: i64) {
        let mut value_ = self.value.write().await;
        *value_ += value;
    }

    async fn is_bigger_than(&self, instance_key: PublicKey, value: i64) -> bool {
        let value_ = self.value.read().await;
        *value_ > value
    }

    async fn get(&self, instance_key: PublicKey) -> i64 {
        let value_ = self.value.read().await;
        *value_
    }

    async fn reset(&self, instance_key: PublicKey) {
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
            }) as Arc<dyn Calculator>),
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
    let zero = PublicKey::zero();
    let client = CalculatorStub::new(Box::new(HttpClient::new("localhost:14123/x".to_owned())));
    client.reset(zero.clone()).await.unwrap();
    client.add(zero.clone(), 1).await.unwrap();
    assert_eq!(client.get(zero.clone(),).await.unwrap(), 1);
    client.add(zero.clone(), 2).await.unwrap();
    assert_eq!(client.get(zero.clone(),).await.unwrap(), 3);
    assert!(!client.is_bigger_than(zero.clone(), 3).await.unwrap());
    assert!(client.is_bigger_than(zero.clone(), 2).await.unwrap());
    client.reset(zero.clone()).await.unwrap();
    assert_eq!(client.get(zero.clone(),).await.unwrap(), 0);
}

#[tokio::main]
async fn main() {
    color_eyre::install().unwrap();
    tokio::task::spawn(server());
    client_trait_aware().await;
}
