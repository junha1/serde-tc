use super::RpcPayload;
use http::{Request, Response, StatusCode};
use hyper::Body;

pub(crate) fn get_header(req: &Request<Body>, key: &str) -> Option<String> {
    req.headers()
        .get(key.to_ascii_lowercase())
        .map(|value| String::from_utf8_lossy(value.as_bytes()).to_string())
}

pub(crate) fn make_response(status_code: StatusCode, msg: String) -> Response<Body> {
    Response::builder()
        .status(status_code)
        .body(Body::from(msg))
        .expect("failed to make response")
}

/// The body will become empty after the function call. If you wish to retain
/// the original body, create a new body by cloning the payload.
/// ```ignore
/// let payload = parse_payload(&mut req);
/// *req.body_mut() = Body::from(serde_json::to_string(&payload).unwrap());
/// ```
pub(crate) async fn parse_payload(req: &mut Request<Body>) -> eyre::Result<RpcPayload> {
    let body_bytes = hyper::body::to_bytes(req.body_mut()).await?;
    println!("{}", String::from_utf8_lossy(&body_bytes));
    let payload = serde_json::from_slice::<RpcPayload>(&body_bytes)?;
    Ok(payload)
}
