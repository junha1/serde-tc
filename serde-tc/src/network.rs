mod auth_client;
mod auth_server;
mod utils;

use crate::network::auth_server::AuthenticationConsumerLayer;

use super::*;
use axum::{
    extract::{Path, State},
    http::{HeaderValue, StatusCode},
    routing::{get, post},
    Json, Router,
};
use hdk_common::crypto::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tower_http::cors::CorsLayer;

const X_HYPERITHM_KEY: &str = "X-HYPERITHM-KEY";
const X_HYPERITHM_SIGNATURE: &str = "X-HYPERITHM-SIGNATURE";

#[derive(Error, Debug)]
enum HttpError {
    #[error("invalid request")]
    InvalidRequest,
    #[error("method not found")]
    MethodNotFound,
}

pub trait HttpInterface:
    DispatchStringDictAsync<Error = serde_json::Error, Poly = serde_json::Value>
    + DispatchStringTupleAsync<Error = serde_json::Error>
    + Send
    + Sync
    + 'static
{
}

impl<T> HttpInterface for Arc<T> where T: HttpInterface + ?Sized {}
pub fn create_http_object<T: ?Sized + HttpInterface>(x: Arc<T>) -> Arc<dyn HttpInterface> {
    Arc::new(x) as Arc<dyn HttpInterface>
}

#[derive(Clone)]
struct AppState {
    pub registered_objects: HashMap<String, Arc<dyn HttpInterface>>,
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "This is a serde-tc JSON RPC server. Please access to /<object-name> with POST, to use the API."
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct RpcPayload {
    pub method: serde_json::value::Value,
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::value::Value>,
}

impl ToHash256 for RpcPayload {
    fn to_hash256(&self) -> Hash256 {
        let bytes = serde_json::to_vec(&self).unwrap();
        Hash256::hash(bytes)
    }
}

async fn dispatch(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Json(args): Json<RawArg>,
) -> (StatusCode, Json<Value>) {
    if let Some(object) = state.registered_objects.get(&path) {
        match dispatch_raw(object.as_ref(), &args.method, args.params.clone()).await {
            Ok(value) => (StatusCode::OK, Json(value)),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "invalid http request",
                    "error_message": err.to_string(),
                    "request": args,
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "object not found",
                "obejct": &path.as_str()[1..],
            })),
        )
    }
}

pub async fn run_server(port: u16, objects: HashMap<String, Arc<dyn HttpInterface>>) {
    let app = Router::new().route("/", get(root));
    let app = app.route("/:key", post(dispatch));
    let app = app
        .layer(AuthenticationConsumerLayer)
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_headers([axum::http::header::CONTENT_TYPE])
                .allow_methods([hyper::Method::POST]),
        )
        .with_state(AppState {
            registered_objects: objects,
        });
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct RawArg {
    method: String,
    params: serde_json::Value,
}

async fn dispatch_raw<T>(
    api: &T,
    method: &str,
    arguments: serde_json::Value,
) -> std::result::Result<serde_json::Value, HttpError>
where
    T: HttpInterface + ?Sized,
{
    let result = if arguments.is_array() {
        DispatchStringTupleAsync::dispatch(api, method, &arguments.to_string()).await
    } else if arguments.is_object() {
        DispatchStringDictAsync::dispatch(api, method, &arguments.to_string()).await
    } else {
        return Err(HttpError::InvalidRequest);
    };

    match result {
        Ok(x) => Ok(serde_json::from_str(&x).unwrap()),
        Err(Error::MethodNotFound(_)) => Err(HttpError::MethodNotFound),
        Err(_) => Err(HttpError::InvalidRequest),
    }
}

/// A RPC client. Use `123.1.2.3:123/object_name` for `addr`.
pub struct HttpClient {
    addr: String,
    private_key: PrivateKey,
}

impl HttpClient {
    pub fn new(addr: String) -> Self {
        HttpClient {
            addr,
            private_key: generate_keypair_random().1,
        }
    }

    pub fn with_auth(addr: String, private_key: PrivateKey) -> Self {
        HttpClient { addr, private_key }
    }
}

#[async_trait]
impl StubCall for HttpClient {
    type Error = eyre::Error;

    async fn call(&self, method: &'static str, params: String) -> Result<String, Self::Error> {
        let body = axum::body::Body::from(format!(
            r#"{{"method": "{}",
        "params": {}}}"#,
            method, params
        ));
        let mut request = axum::http::Request::builder()
            .method(hyper::Method::POST)
            .uri(&format!("http://{}", self.addr))
            .header("content-type", "application/json")
            .body(body)
            .expect("trivial");
        auth_client::add_auth(&mut request, &self.private_key).await?;
        use tower::{Service, ServiceBuilder, ServiceExt};
        let mut client = ServiceBuilder::new().service(hyper::Client::new());
        let cli = client.ready().await.unwrap();
        let response = cli.call(request).await.unwrap();
        let status = response.status().as_u16();
        let body_text = String::from_utf8(hyper::body::to_bytes(response).await?.to_vec())?;
        if status != 200 {
            Err(eyre::Error::msg(format!(
                r#"HTTP request failed: "{}""#,
                body_text
            )))
        } else {
            Ok(body_text)
        }
    }
}
