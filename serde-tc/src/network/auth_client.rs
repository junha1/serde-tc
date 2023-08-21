//! This middleware is for the client. It holds a private key and adds the
//! public key and signature for the payload in headers. Additionally, it
//! removes the first parameter of the payload which should be a dummy public
//! key value, acting as a placeholder.

use std::{
    error::Error,
    task::{Context, Poll},
};

use futures::future::BoxFuture;
use hdk_common::{
    crypto::{PrivateKey, Signature, ToHash256},
    serde_hdk,
};
use http::{HeaderValue, Request, Response, StatusCode};
use hyper::Body;
use tower::{Layer, Service};

use super::{
    utils::{make_response, parse_payload},
    X_HYPERITHM_KEY, X_HYPERITHM_SIGNATURE,
};

/// Middleware layer that adds authentication headers to the HTTP requests
#[derive(Debug, Clone)]
pub struct AuthenticationProviderLayer {
    private_key: PrivateKey,
}

impl AuthenticationProviderLayer {
    pub fn new(private_key: PrivateKey) -> Self {
        Self { private_key }
    }
}

impl<S> Layer<S> for AuthenticationProviderLayer {
    type Service = AuthenticationProvider<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthenticationProvider::new(self.private_key.clone(), inner)
    }
}

/// Middleware service that adds authentication headers to the HTTP requests
/// generated by `AuthenticationProviderLayer`
#[derive(Debug, Clone)]
pub struct AuthenticationProvider<S> {
    private_key: PrivateKey,
    inner: S,
}

impl<S> AuthenticationProvider<S> {
    pub(self) fn new(private_key: PrivateKey, inner: S) -> Self {
        Self { private_key, inner }
    }

    async fn add_auth(req: &mut Request<Body>, private_key: PrivateKey) -> Result<(), String> {
        let payload = parse_payload(req).await.map_err(|err| err.to_string())?;
        *req.body_mut() = Body::from(serde_json::to_string(&payload).unwrap());

        let signature =
            Signature::sign(payload.to_hash256(), &private_key).map_err(|err| err.to_string())?;
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
}

impl<S> Service<Request<Body>> for AuthenticationProvider<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + std::marker::Send + Clone + 'static,
    S::Response: 'static,
    S::Error: Into<Box<dyn Error + Send + Sync>> + 'static,
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
        let private_key = self.private_key.clone();
        let res_fut = async move {
            match Self::add_auth(&mut req, private_key).await {
                Ok(_) => inner.call(req).await,
                Err(err) => Ok(make_response(StatusCode::UNAUTHORIZED, err)),
            }
        };
        Box::pin(res_fut)
    }
}

#[cfg(never)]
#[cfg(test)]
mod tests {
    use hdk_common::crypto::generate_keypair;
    use tower::{ServiceBuilder, ServiceExt};

    use super::*;
    use crate::network::RpcPayload;

    #[tokio::test]
    async fn test_add_auth() {
        let (_, private_key) = generate_keypair("hello world");
        let add_auth = AuthenticationProviderLayer::new(private_key);
        let mut svc =
            ServiceBuilder::new()
                .layer(add_auth)
                .service_fn(|_: Request<Body>| async move {
                    Ok::<_, hyper::Error>(Response::new(Body::empty()))
                });

        let payload = RpcPayload {
            method: serde_json::Value::String("foo".to_owned()),
            params: vec![serde_json::Value::String("dummy".to_owned())],
        };
        let request = Request::builder()
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();

        let response = svc.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

pub async fn add_auth(
    req: &mut Request<Body>,
    private_key: &PrivateKey,
) -> Result<(), eyre::Error> {
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
