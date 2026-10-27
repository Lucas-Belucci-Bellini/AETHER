use std::ops::Deref;
use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use liga_core::{AppContext, Validate};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::ApiError;

/// JSON body extractor/response whose rejections use [`ApiError`].
///
/// - malformed JSON: `400 malformed_json`
/// - wrong shape/types: `422 invalid_body`
/// - missing `Content-Type: application/json`: `415 unsupported_media_type`
/// - body over the configured limit: `413 payload_too_large`
#[derive(Debug, Clone, Copy, Default)]
pub struct Json<T>(pub T);

fn json_rejection(rejection: JsonRejection) -> ApiError {
    match rejection.status() {
        StatusCode::PAYLOAD_TOO_LARGE => ApiError::payload_too_large(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE => {
            ApiError::unsupported_media_type("expected `Content-Type: application/json`")
        }
        StatusCode::UNPROCESSABLE_ENTITY => ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_body",
            rejection.body_text(),
        ),
        _ => ApiError::new(
            StatusCode::BAD_REQUEST,
            "malformed_json",
            rejection.body_text(),
        ),
    }
}

impl<T, S> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(json_rejection(rejection)),
        }
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

/// Like [`Json`], then runs [`Validate`]; failures become `422 validation_failed`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate().map_err(ApiError::validation)?;
        Ok(Self(value))
    }
}

/// Path parameters; rejections become `400 invalid_path`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Path<T>(pub T);

impl<T, S> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Path::<T>::from_request_parts(parts, state).await {
            Ok(axum::extract::Path(value)) => Ok(Self(value)),
            Err(rejection) => Err(path_rejection(rejection)),
        }
    }
}

fn path_rejection(rejection: PathRejection) -> ApiError {
    if rejection.status().is_server_error() {
        ApiError::internal(rejection)
    } else {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_path",
            rejection.body_text(),
        )
    }
}

/// Query string; rejections become `400 invalid_query`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

impl<T, S> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Query::<T>::from_request_parts(parts, state).await {
            Ok(axum::extract::Query(value)) => Ok(Self(value)),
            Err(rejection) => Err(query_rejection(rejection)),
        }
    }
}

fn query_rejection(rejection: QueryRejection) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "invalid_query",
        rejection.body_text(),
    )
}

/// Typed shared state provided by a module (see `ModuleContext::provide`).
///
/// A missing value is a wiring bug and yields `500 internal_error`.
#[derive(Debug)]
pub struct Dep<T>(pub Arc<T>);

impl<T> Clone for Dep<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for Dep<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Send + Sync + 'static> FromRequestParts<AppContext> for Dep<T> {
    type Rejection = ApiError;

    async fn from_request_parts(
        _parts: &mut Parts,
        ctx: &AppContext,
    ) -> Result<Self, Self::Rejection> {
        ctx.require::<T>().map(Dep).map_err(ApiError::internal)
    }
}
