use async_trait::async_trait;
use http::*;
use reqwest::Client;
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
impl Calculator for SimpleCalculator {
    async fn add(&self, value: i64) {
        let mut value_ = self.value.write().await;
        *value_ += value;
    }

    async fn is_bigger_than(&self, value: i64) -> bool {
        let value_ = self.value.read().await;
        *value_ > value
    }

    async fn get(&self) -> i64 {
        let value_ = self.value.read().await;
        *value_
    }

    async fn reset(&self) {
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
    let client = CalculatorStub::new(Box::new(HttpClient::new(
        "localhost:14123/x".to_owned(),
        Client::new(),
    )));
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

/// Client side case 1;
/// You only 'know' the definition of `trait Calculator` and all you can do is accessing using HTTP
/// This way of accessing the server is language-agnostic; it uses only JSON and HTTP!
async fn client_trait_unaware() {
    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:14123/x")
        .header("content-type", "application/json")
        .body(r#"{"method": "reset", "params": {}}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(response.json::<Value>().await.unwrap().is_null());

    let response = client
        .post("http://localhost:14123/x")
        .header("content-type", "application/json")
        .body(r#"{"method": "add", "params": {"value": 5}}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(response.json::<Value>().await.unwrap().is_null());

    let response = client
        .post("http://localhost:14123/x")
        .header("content-type", "application/json")
        .body(r#"{"method": "is_bigger_than", "params": {"value": 4}}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(response.json::<bool>().await.unwrap());
}

#[tokio::main]
async fn main() {
    tokio::task::spawn(server());

    client_trait_aware().await;
    client_trait_unaware().await;
}
