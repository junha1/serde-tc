use super::*;
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

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
struct State {
    pub registered_objects: HashMap<String, Arc<dyn HttpInterface>>,
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "This is a serde-tc JSON RPC server. Please access to /<object-name> with POST, to use the API."
}

async fn dispatch(
    Path(path): Path<String>,
    Json(args): Json<RawArg>,
    Extension(state): Extension<Arc<State>>,
) -> (StatusCode, Json<Value>) {
    println!("{:?} | {:?}", path, args);
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
    let app = app.layer(Extension(Arc::new(State {
        registered_objects: objects,
    })));
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
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
    println!("{:?} | {:?}", method, arguments);
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
    client: Client,
    addr: String,
}

impl HttpClient {
    pub fn new(addr: String, client: Client) -> Self {
        HttpClient { client, addr }
    }
}

#[async_trait]
impl StubCall for HttpClient {
    type Error = anyhow::Error;

    async fn call(&self, method: &'static str, params: String) -> Result<String, Self::Error> {
        let body = format!(
            r#"{{"method": "{}",
        "params": {}}}"#,
            method, params
        );
        let response = self
            .client
            .request(Method::POST, &format!("http://{}", self.addr))
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await?;

        if response.status().as_u16() != 200 {
            Err(anyhow::Error::msg(format!(
                r#"HTTP request failed: "{}""#,
                response.text().await?
            )))
        } else {
            Ok(response.text().await?)
        }
    }
}
