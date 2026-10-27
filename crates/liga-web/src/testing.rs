//! Drive a router in tests without binding a port.
//!
//! Helpers here panic on misuse (invalid URIs, non-JSON bodies) because they
//! are meant for test code, where a panic is a clear failure.
#![allow(clippy::expect_used)]

use axum::Router;
use axum::body::{Body, Bytes};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tower::ServiceExt;

/// In-process HTTP client for a [`Router`].
#[derive(Debug, Clone)]
pub struct TestClient {
    router: Router,
}

impl TestClient {
    /// Wraps a fully built router (e.g. from [`crate::build_router`]).
    pub fn new(router: Router) -> Self {
        Self { router }
    }

    /// `GET uri`.
    pub async fn get(&self, uri: &str) -> TestResponse {
        self.send(
            Self::request(Method::GET, uri)
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
    }

    /// `GET uri` with one extra header.
    pub async fn get_with_header(&self, uri: &str, name: &str, value: &str) -> TestResponse {
        let request = Self::request(Method::GET, uri)
            .header(name, value)
            .body(Body::empty())
            .expect("valid request");
        self.send(request).await
    }

    /// `POST uri` with a JSON body.
    pub async fn post_json(&self, uri: &str, body: &impl Serialize) -> TestResponse {
        let json = serde_json::to_vec(body).expect("serializable body");
        self.post_raw_bytes(uri, "application/json", json).await
    }

    /// `POST uri` with an arbitrary content type and body.
    pub async fn post_raw(&self, uri: &str, content_type: &str, body: &str) -> TestResponse {
        self.post_raw_bytes(uri, content_type, body.as_bytes().to_vec())
            .await
    }

    async fn post_raw_bytes(&self, uri: &str, content_type: &str, body: Vec<u8>) -> TestResponse {
        let request = Self::request(Method::POST, uri)
            .header(CONTENT_TYPE, content_type)
            .body(Body::from(body))
            .expect("valid request");
        self.send(request).await
    }

    /// Sends any request.
    pub async fn send(&self, request: Request<Body>) -> TestResponse {
        let response = match self.router.clone().oneshot(request).await {
            Ok(response) => response,
            Err(never) => match never {},
        };
        let (parts, body) = response.into_parts();
        let body = axum::body::to_bytes(body, usize::MAX)
            .await
            .expect("readable response body");
        TestResponse {
            status: parts.status,
            headers: parts.headers,
            body,
        }
    }

    fn request(method: Method, uri: &str) -> axum::http::request::Builder {
        Request::builder().method(method).uri(uri)
    }
}

/// A fully buffered response.
#[derive(Debug, Clone)]
pub struct TestResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl TestResponse {
    /// Status code.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// All headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// One header as text.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Raw body.
    pub fn bytes(&self) -> &Bytes {
        &self.body
    }

    /// Body as (lossy) UTF-8.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// Body parsed as JSON. Panics with the body text if it is not valid JSON for `T`.
    #[track_caller]
    pub fn json<T: DeserializeOwned>(&self) -> T {
        match serde_json::from_slice(&self.body) {
            Ok(value) => value,
            Err(err) => panic!(
                "response body is not the expected JSON ({err}): {}",
                self.text()
            ),
        }
    }
}
