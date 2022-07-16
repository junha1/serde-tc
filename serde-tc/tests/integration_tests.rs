use http::*;
use reqwest::Client;
use serde_tc::*;
use std::sync::Arc;

#[serde_tc(dispatcher, encoder, dict, tuple)]
trait Trait1 {
    fn f1(&self, a1: i64, a2: &str, a3: &i32) -> String;
    fn f2(&self) -> String;
    fn f3(&self, a1: i32);
}

#[serde_tc_full]
trait Trait2: Send + Sync {
    async fn f1(&self, a1: i64, a2: &str, a3: &i32) -> String;
    async fn f2(&self) -> String;
    async fn f3(&self, a1: i32);
}

struct SimpleImpl;

impl Trait1 for SimpleImpl {
    fn f1(&self, a1: i64, a2: &str, a3: &i32) -> String {
        format!("{}{}{}", a1, a2, a3)
    }
    fn f2(&self) -> String {
        "hi".to_owned()
    }
    fn f3(&self, _a1: i32) {}
}

#[async_trait::async_trait]
impl Trait2 for SimpleImpl {
    async fn f1(&self, a1: i64, a2: &str, a3: &i32) -> String {
        format!("{}{}{}", a1, a2, a3)
    }
    async fn f2(&self) -> String {
        "hi".to_owned()
    }
    async fn f3(&self, _a1: i32) {}
}

#[test]
fn test1() {
    let object = SimpleImpl;
    let object_ref = &object as &dyn Trait1;

    let args = trait1_encoder_tuple::f1(1, "hello", &3);
    assert_eq!(
        DispatchStringTuple::dispatch(object_ref, "f1", &args).unwrap(),
        format!(r#""{}{}{}""#, 1, "hello", 3)
    );
    let args = trait1_encoder_dict::f1(1, "hello", &3);
    assert_eq!(
        DispatchStringDict::dispatch(object_ref, "f1", &args).unwrap(),
        format!(r#""{}{}{}""#, 1, "hello", 3)
    );
}

#[tokio::test]
async fn test1_async() {
    let object = SimpleImpl;
    let object_ref = &object as &dyn Trait2;

    let args = trait2_encoder_tuple::f1(1, "hello", &3);
    assert_eq!(
        DispatchStringTupleAsync::dispatch(object_ref, "f1", &args)
            .await
            .unwrap(),
        format!(r#""{}{}{}""#, 1, "hello", 3)
    );
    let args = trait2_encoder_dict::f1(1, "hello", &3);
    assert_eq!(
        DispatchStringDictAsync::dispatch(object_ref, "f1", &args)
            .await
            .unwrap(),
        format!(r#""{}{}{}""#, 1, "hello", 3)
    );
}

impl HttpInterface for dyn Trait2 {}

fn create_server(port: u16) {
    tokio::task::spawn(run_server(
        port,
        [(
            "x".to_owned(),
            create_http_object(Arc::new(SimpleImpl) as Arc<dyn Trait2>),
        )]
        .iter()
        .cloned()
        .collect(),
    ));
}

#[tokio::test]
async fn test_success() {
    create_server(4000);
    let client = Trait2Stub::new(Box::new(HttpClient::new(
        "localhost:4000/x".to_owned(),
        Client::new(),
    )));
    let res = client.f1(1, "2", &3).await.unwrap();
    assert_eq!(res, "123");
}

#[tokio::test]
async fn test_failure0() {
    create_server(3000);
    let client = Trait2Stub::new(Box::new(HttpClient::new(
        "localhost:432/x".to_owned(),
        Client::new(),
    )));
    let res = client.f1(1, "2", &3).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_success_http() {
    create_server(4001);
    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:4001/x")
        .header("content-type", "application/json")
        .body(r#"{"method": "f1", "params": [1, "2", 3]}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.json::<String>().await.unwrap(), "123");
}

#[tokio::test]
async fn test_failure1() {
    create_server(4002);
    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:4002/x")
        .header("content-type", "application/json")
        .body(r#"{"method": "nonexistent-method", "params": {}}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn test_failure2() {
    create_server(4003);
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:4003/x")
        .header("content-type", "application/json")
        .body(r#"This request is not a valid JSON"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_failure3() {
    create_server(4003);
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:4003/missing")
        .header("content-type", "application/json")
        .body(r#"{"method": "doesn't matter", "params": {}}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_failure4() {
    create_server(4005);
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:4005/x")
        .header("content-type", "application/json")
        .body(r#""This is a JSON but doesn't match the parameter type""#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
}
