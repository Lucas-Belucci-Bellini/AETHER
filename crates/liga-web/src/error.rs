use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use liga_core::ValidationErrors;
use serde::Serialize;
use serde_json::Value;

use crate::middleware::current_request_id;

/// `Result` alias for handlers.
pub type ApiResult<T> = Result<T, ApiError>;

/// An HTTP error rendered as:
///
/// ```json
/// { "error": { "code": "not_found", "message": "product 7 not found",
///              "details": { ... }, "request_id": "..." } }
/// ```
///
/// Any `std::error::Error` converts into a `500 internal_error` via `?`; the
/// original error is logged with the request ID and never sent to the client.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: Cow<'static, str>,
    message: Cow<'static, str>,
    details: Option<Value>,
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl ApiError {
    /// Creates an error with an explicit status, stable code and client-safe message.
    pub fn new(
        status: StatusCode,
        code: impl Into<Cow<'static, str>>,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            details: None,
            source: None,
        }
    }

    /// `400 bad_request`.
    pub fn bad_request(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    /// `401 unauthorized`.
    pub fn unauthorized(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", message)
    }

    /// `403 forbidden`.
    pub fn forbidden(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    /// `404 not_found`.
    pub fn not_found(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    /// `405 method_not_allowed`.
    pub fn method_not_allowed() -> Self {
        Self::new(
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
            "method not allowed for this route",
        )
    }

    /// `409 conflict`.
    pub fn conflict(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    /// `413 payload_too_large`.
    pub fn payload_too_large() -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "request body is too large",
        )
    }

    /// `415 unsupported_media_type`.
    pub fn unsupported_media_type(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_media_type",
            message,
        )
    }

    /// `422 validation_failed` with the failed rules under `details.fields`.
    pub fn validation(errors: ValidationErrors) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "request validation failed",
        )
        .with_details(serde_json::json!({ "fields": errors }))
    }

    /// `429 too_many_requests`.
    pub fn too_many_requests(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, "too_many_requests", message)
    }

    /// `500 internal_error`. `source` is logged, never returned to the client.
    pub fn internal(source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "internal server error",
        )
        .with_source(source)
    }

    /// `503 service_unavailable`.
    pub fn service_unavailable(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
            message,
        )
    }

    /// `503 timeout`: the request exceeded the configured time limit.
    pub fn timeout() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "timeout",
            "the request took too long to complete",
        )
    }

    /// Attaches client-visible structured details.
    pub fn with_details(mut self, details: impl Serialize) -> Self {
        self.details = serde_json::to_value(details).ok();
        self
    }

    /// Attaches an internal cause (logged, never returned).
    pub fn with_source(mut self, source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// HTTP status.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Stable machine-readable code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Client-safe message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}: {}",
            self.status.as_u16(),
            self.code,
            self.message
        )
    }
}

impl<E: StdError + Send + Sync + 'static> From<E> for ApiError {
    fn from(err: E) -> Self {
        Self::internal(err)
    }
}

#[derive(Serialize)]
struct Envelope<'a> {
    error: Body<'a>,
}

#[derive(Serialize)]
struct Body<'a> {
    code: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<&'a str>,
}

fn chain(source: &(dyn StdError + 'static)) -> String {
    let mut out = source.to_string();
    let mut next = source.source();
    while let Some(err) = next {
        out.push_str(": ");
        out.push_str(&err.to_string());
        next = err.source();
    }
    out
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let request_id = current_request_id();
        let cause = self
            .source
            .as_deref()
            .map(|s| chain(s as &(dyn StdError + 'static)));
        if self.status.is_server_error() {
            tracing::error!(
                status = self.status.as_u16(),
                code = %self.code,
                cause = cause.as_deref().unwrap_or("-"),
                "request failed"
            );
        } else {
            tracing::debug!(status = self.status.as_u16(), code = %self.code, "request rejected");
        }
        let body = Envelope {
            error: Body {
                code: &self.code,
                message: &self.message,
                details: self.details.as_ref(),
                request_id: request_id.as_ref().map(|id| id.as_str()),
            },
        };
        (self.status, axum::Json(body)).into_response()
    }
}
