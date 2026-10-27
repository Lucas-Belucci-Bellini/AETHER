//! # liga-web
//!
//! HTTP layer of Liga, built on [Axum](https://docs.rs/axum) and Tokio.
//!
//! - [`Routes`]: what a module contributes; handlers receive the
//!   [`AppContext`](liga_core::AppContext) as Axum state.
//! - [`ApiError`]: one JSON error shape for every failure, including 404/405,
//!   malformed bodies, timeouts and internal errors (whose details are logged,
//!   never returned).
//! - [`ApiEnvelope`] and [`ApiMeta`]: typed success envelope for the family
//!   versioned HTTP/JSON contract.
//! - Extractors with consistent rejections: [`Json`], [`ValidJson`],
//!   [`Path`], [`Query`] and [`Dep`].
//! - [`build_router`]: applies request IDs, tracing, a per-request timeout,
//!   a global concurrency limit and a body-size limit.
//! - [`testing::TestClient`]: drive the router in tests without opening a port.
//!
//! Axum types are re-exported as [`axum`] and are part of this crate's public
//! API: a breaking Axum upgrade is a breaking `liga-web` release.

mod contract;
mod error;
mod extract;
mod middleware;
mod routes;
mod server;
pub mod testing;

pub use axum;
pub use contract::{ApiEnvelope, ApiMeta};
pub use error::{ApiError, ApiResult};
pub use extract::{Dep, Json, Path, Query, ValidJson};
pub use middleware::{REQUEST_ID_HEADER, RequestId, current_request_id};
pub use routes::Routes;
pub use server::{ServerConfig, WebError, build_router, serve};
