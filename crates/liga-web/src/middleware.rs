use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{FromRequestParts, MatchedPath, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tokio::sync::Semaphore;
use tracing::Instrument;

use crate::error::ApiError;

/// Header used to read and return the request ID.
pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

const MAX_REQUEST_ID_LEN: usize = 64;

tokio::task_local! {
    static CURRENT_REQUEST_ID: RequestId;
}

/// Correlation ID of the current request.
///
/// An incoming `x-request-id` is reused only if it is 1–64 characters of
/// `[A-Za-z0-9._-]`; otherwise a new UUID is generated. The ID is returned in
/// the response header, included in error bodies and attached to the
/// request's tracing span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestId(Arc<str>);

impl RequestId {
    /// The ID as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string().into())
    }

    fn from_header(value: &HeaderValue) -> Option<Self> {
        let value = value.to_str().ok()?;
        let valid = !value.is_empty()
            && value.len() <= MAX_REQUEST_ID_LEN
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        valid.then(|| Self(value.into()))
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for RequestId {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<RequestId>().cloned().ok_or_else(|| {
            ApiError::internal(std::io::Error::other(
                "RequestId extension missing: router was not built with liga_web::build_router",
            ))
        })
    }
}

/// Request ID of the request being handled by the current task, if any.
pub fn current_request_id() -> Option<RequestId> {
    CURRENT_REQUEST_ID.try_with(Clone::clone).ok()
}

/// Outermost middleware: request ID, tracing span and completion log.
pub(crate) async fn request_context(mut req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(RequestId::from_header)
        .unwrap_or_else(RequestId::generate);
    req.extensions_mut().insert(id.clone());

    let method = req.method().clone();
    // The route template (not the raw path/query) keeps logs low-cardinality
    // and free of values passed in the query string.
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| "<unmatched>".to_owned(), |p| p.as_str().to_owned());
    let span = tracing::info_span!("http.request", %method, %route, request_id = %id);
    let started = Instant::now();

    let mut response = CURRENT_REQUEST_ID
        .scope(id.clone(), next.run(req))
        .instrument(span.clone())
        .await;

    if let Ok(value) = HeaderValue::from_str(id.as_str()) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    let status = response.status().as_u16();
    let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    span.in_scope(|| {
        if status >= 500 {
            tracing::warn!(status, latency_ms, "request completed");
        } else {
            tracing::info!(status, latency_ms, "request completed");
        }
    });
    response
}

/// Shared limits enforced for every request.
pub(crate) struct Guard {
    pub(crate) permits: Arc<Semaphore>,
    pub(crate) timeout: Duration,
}

/// Global concurrency limit + per-request timeout (waiting for a permit counts).
pub(crate) async fn guard(State(guard): State<Arc<Guard>>, req: Request, next: Next) -> Response {
    let work = async {
        let _permit = guard.permits.clone().acquire_owned().await.ok();
        next.run(req).await
    };
    match tokio::time::timeout(guard.timeout, work).await {
        Ok(response) => response,
        Err(_) => {
            let timeout_ms = u64::try_from(guard.timeout.as_millis()).unwrap_or(u64::MAX);
            tracing::warn!(timeout_ms, "request timed out");
            ApiError::timeout().into_response()
        }
    }
}
