mod auth;

use super::*;
use auth::AuthError;
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
enum NetworkError {
    #[error("failed to parse request: {0}")]
    Dispatch(DispatchError<serde_json::Error>),
    #[error("failed to authenticate: {0}")]
    Auth(#[from] AuthError),
    #[error("failed to find object: {0}")]
    ObjectNotFound(String),
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown error of code {0}: {1}")]
    Unknown(u16, String),
}

pub trait HttpInterface:
    DispatchStringDictAsync<Error = serde_json::Error, Poly = serde_json::Value> + Send + Sync + 'static
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct RawArg {
    method: String,
    params: serde_json::Value,
}

async fn dispatch(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Json(args): Json<RawArg>,
) -> (StatusCode, Json<Value>) {
    if let Some(object) = state.registered_objects.get(&path) {
        let result = DispatchStringDictAsync::dispatch(
            object.as_ref(),
            &args.method,
            &args.params.to_string(),
        )
        .await;
        match result {
            Ok(value) => (StatusCode::OK, Json(serde_json::from_str(&value).unwrap())),
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
        .layer(auth::AuthenticationConsumerLayer)
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
    type Error = NetworkError;

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
        auth::add_auth(&mut request, &self.private_key).await?;
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
