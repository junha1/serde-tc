use super::*;
use futures::future::BoxFuture;
use hdk_common::crypto::*;
use hdk_common::{
    crypto::{PrivateKey, Signature, ToHash256},
    serde_hdk,
};
use http::{HeaderValue, Request, Response, StatusCode};
use hyper::Body;
use std::task::{Context, Poll};
use thiserror::Error;
use tower::{Layer, Service};

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("crypto error: {0}")]
    Crypto(#[from] hdk_common::crypto::CryptoError),
    #[error("protocol error: {0}")]
    Protocol(String),
}

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
pub(crate) async fn parse_payload(req: &mut Request<Body>) -> Result<RpcPayload, AuthError> {
    let body_bytes = hyper::body::to_bytes(req.body_mut()).await?;
    println!("{}", String::from_utf8_lossy(&body_bytes));
    let payload = serde_json::from_slice::<RpcPayload>(&body_bytes)?;
    Ok(payload)
}

/// Add a signature to the request body and insert the public key in the header.
pub async fn add_auth(req: &mut Request<Body>, private_key: &PrivateKey) -> Result<(), AuthError> {
    let payload = parse_payload(req).await?;
    *req.body_mut() = Body::from(serde_json::to_string(&payload).unwrap());

    let signature = Signature::sign(payload.to_hash256(), &private_key)?;
    req.headers_mut().insert(
        X_HYPERITHM_KEY,
        HeaderValue::from_str(
            &serde_hdk::to_string(&private_key.public_key())
                .expect("failed to serialize public key"),
        )
        .expect("failed to insert auth key in header"),
    );
    req.headers_mut().insert(
        X_HYPERITHM_SIGNATURE,
        HeaderValue::from_str(
            &serde_hdk::to_string(&signature).expect("failed to serialize signature"),
        )
        .expect("failed to insert auth key in header"),
    );
    Ok(())
}

/// Structure to hold authentication information extracted from the request.
struct AuthInfo {
    signature: TypedSignature<RpcPayload>,
    payload: RpcPayload,
}

impl AuthInfo {
    pub fn new(public_key: PublicKey, signature: Signature, payload: RpcPayload) -> Self {
        Self {
            signature: TypedSignature::new(signature, public_key),
            payload,
        }
    }

    pub fn verify(&self) -> Result<(), hdk_common::crypto::CryptoError> {
        self.signature.verify(&self.payload)
    }
}

/// Authentication middleware layer.
///
/// This layer extracts authentication information from the request headers,
/// verifies the signature for the provided payload, and injects the public key
/// as the first parameter in the RPC payload.
#[derive(Debug, Clone)]
pub struct AuthenticationConsumerLayer;

impl<S> Layer<S> for AuthenticationConsumerLayer {
    type Service = AuthenticationConsumer<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthenticationConsumer::new(inner)
    }
}

/// Authentication middleware service generated by
/// `AuthenticationConsumerLayer`.
///
/// This middleware performs the authentication and verification process before
/// passing the request to the inner service. If the authentication fails, it
/// responds with an unauthorized error.
#[derive(Debug, Clone)]
pub struct AuthenticationConsumer<S> {
    inner: S,
}

impl<S> AuthenticationConsumer<S> {
    pub(self) fn new(inner: S) -> Self {
        Self { inner }
    }

    async fn authenticate(req: &mut Request<Body>) -> Result<(), AuthError>
    where
        S: Service<Request<Body>, Response = axum::response::Response>
            + std::marker::Send
            + Clone
            + 'static,
    {
        let auth_info = Self::extract_auth_info(req).await?;
        auth_info.verify().map_err(|err| err.into())
    }

    async fn extract_auth_info(req: &mut Request<Body>) -> Result<AuthInfo, AuthError> {
        let public_key: PublicKey = serde_hdk::from_str(
            &get_header(req, X_HYPERITHM_KEY)
                .ok_or_else(|| AuthError::Protocol("falied to get public key".to_owned()))?,
        )?;
        let signature: Signature = serde_hdk::from_str(
            &get_header(req, X_HYPERITHM_SIGNATURE)
                .ok_or_else(|| AuthError::Protocol("falied to get signature".to_owned()))?,
        )?;

        let mut payload = parse_payload(req).await?;
        let auth_info = AuthInfo::new(public_key.clone(), signature, payload.clone());
        payload.params.insert(
            "caller_key".to_owned(),
            serde_json::to_value(&public_key).expect("failed to serialize public key"),
        );
        let new_body =
            Body::from(serde_json::to_string(&payload).expect("failed to serialize payload"));
        *req.body_mut() = new_body;
        Ok(auth_info)
    }
}

impl<S> Service<Request<Body>> for AuthenticationConsumer<S>
where
    S: Service<Request<Body>, Response = axum::response::Response>
        + std::marker::Send
        + Clone
        + 'static,
    S::Response: 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let res_fut = async move {
            match Self::authenticate(&mut req).await {
                Ok(_) => inner.call(req).await,
                Err(err) => Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(axum::body::boxed(Body::from(err.to_string())))
                    .expect("plain response creation")),
            }
        };
        Box::pin(res_fut)
    }
}
