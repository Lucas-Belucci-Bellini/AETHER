use std::net::{Ipv4Addr, SocketAddr};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use liga_core::{AppContext, Shutdown, Validate, ValidationErrors};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

use crate::error::ApiError;
use crate::middleware::{self, Guard};
use crate::routes::Routes;

/// The `[server]` configuration section.
///
/// ```toml
/// [server]
/// addr = "127.0.0.1:3000"
/// request_timeout_ms = 10000
/// body_limit_bytes = 1048576
/// max_concurrent_requests = 512
/// shutdown_timeout_ms = 10000
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    /// Listen address. Default `127.0.0.1:3000` (local only).
    pub addr: SocketAddr,
    /// Per-request time limit, including waiting for a concurrency permit.
    pub request_timeout_ms: u64,
    /// Maximum request body size.
    pub body_limit_bytes: usize,
    /// Requests processed at the same time; the rest wait (within the timeout).
    pub max_concurrent_requests: usize,
    /// Time allowed for in-flight requests to finish after shutdown starts.
    pub shutdown_timeout_ms: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 3000)),
            request_timeout_ms: 10_000,
            body_limit_bytes: 1024 * 1024,
            max_concurrent_requests: 512,
            shutdown_timeout_ms: 10_000,
        }
    }
}

impl ServerConfig {
    /// [`ServerConfig::request_timeout_ms`] as a `Duration`.
    pub fn request_timeout(&self) -> Duration {
        Duration::from_millis(self.request_timeout_ms)
    }

    /// [`ServerConfig::shutdown_timeout_ms`] as a `Duration`.
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_millis(self.shutdown_timeout_ms)
    }
}

impl Validate for ServerConfig {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors
            .check(
                (1..=600_000).contains(&self.request_timeout_ms),
                "request_timeout_ms",
                "range",
                "must be between 1 and 600000 (10 minutes)",
            )
            .check(
                (1..=64 * 1024 * 1024).contains(&self.body_limit_bytes),
                "body_limit_bytes",
                "range",
                "must be between 1 and 67108864 (64 MiB)",
            )
            .check(
                (1..=Semaphore::MAX_PERMITS).contains(&self.max_concurrent_requests),
                "max_concurrent_requests",
                "range",
                "must be at least 1",
            )
            .check(
                self.shutdown_timeout_ms <= 600_000,
                "shutdown_timeout_ms",
                "range",
                "must be at most 600000 (10 minutes)",
            );
        errors.into_result()
    }
}

/// Errors detected while assembling the HTTP layer.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WebError {
    /// Two route sets declare the same path/method.
    #[error("conflicting routes: {0}")]
    RouteConflict(String),
}

async fn not_found() -> ApiError {
    ApiError::not_found("route not found")
}

async fn method_not_allowed() -> ApiError {
    ApiError::method_not_allowed()
}

/// Merges route sets and applies Liga's default middleware stack
/// (outermost first): request ID + tracing, concurrency limit + timeout,
/// body-size limit. Unknown routes and methods return JSON errors.
pub fn build_router(
    ctx: AppContext,
    routes: impl IntoIterator<Item = Routes>,
    config: &ServerConfig,
) -> Result<Router, WebError> {
    let routes: Vec<Routes> = routes.into_iter().collect();
    let merged = catch_unwind(AssertUnwindSafe(move || {
        routes
            .into_iter()
            .fold(Router::<AppContext>::new(), |acc, r| {
                acc.merge(r.into_router())
            })
    }))
    .map_err(|panic| {
        let message = panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_else(|| "route sets overlap".to_owned());
        WebError::RouteConflict(message)
    })?;

    let guard = Arc::new(Guard {
        permits: Arc::new(Semaphore::new(config.max_concurrent_requests)),
        timeout: config.request_timeout(),
    });

    Ok(merged
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .layer(DefaultBodyLimit::max(config.body_limit_bytes))
        .layer(axum::middleware::from_fn_with_state(
            guard,
            middleware::guard,
        ))
        .layer(axum::middleware::from_fn(middleware::request_context))
        .with_state(ctx))
}

/// Serves `router` until `shutdown` triggers, then stops accepting
/// connections and waits for in-flight requests.
pub async fn serve(
    listener: TcpListener,
    router: Router,
    shutdown: Shutdown,
) -> std::io::Result<()> {
    axum::serve(listener, router)
        .with_graceful_shutdown(async move { shutdown.wait().await })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::{Dep, Json, Path, ValidJson};
    use crate::testing::TestClient;
    use axum::http::StatusCode;
    use axum::routing::{get, post};
    use liga_core::{Config, StateMap};
    use serde::Deserialize;
    use serde_json::{Value, json};

    #[derive(Deserialize)]
    struct NewItem {
        name: String,
    }

    impl Validate for NewItem {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();
            errors.check(
                !self.name.trim().is_empty(),
                "name",
                "required",
                "name must not be empty",
            );
            errors.into_result()
        }
    }

    struct Greeting(&'static str);

    fn ctx() -> AppContext {
        let mut state = StateMap::new();
        state.insert(Greeting("hello")).unwrap();
        AppContext::new("test", Config::default(), state, Shutdown::new())
    }

    fn routes() -> Routes {
        Routes::new()
            .route("/greet", get(|Dep(g): Dep<Greeting>| async move { g.0 }))
            .route(
                "/missing-dep",
                get(|_: Dep<String>| async { "unreachable" }),
            )
            .route(
                "/items",
                post(|ValidJson(item): ValidJson<NewItem>| async move {
                    Json(json!({"name": item.name}))
                }),
            )
            .route(
                "/items/{id}",
                get(|Path(id): Path<u32>| async move { Json(json!({"id": id})) }),
            )
            .route(
                "/echo",
                post(|Json(body): Json<Value>| async move { Json(body) }),
            )
            .route(
                "/fail",
                get(|| async {
                    Err::<(), ApiError>(std::io::Error::other("db password=hunter2").into())
                }),
            )
            .route(
                "/slow",
                get(|| async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    "late"
                }),
            )
    }

    fn client() -> TestClient {
        let config = ServerConfig {
            request_timeout_ms: 50,
            body_limit_bytes: 64,
            ..ServerConfig::default()
        };
        TestClient::new(build_router(ctx(), [routes()], &config).unwrap())
    }

    fn error_code(body: &Value) -> &str {
        body["error"]["code"].as_str().unwrap()
    }

    #[tokio::test]
    async fn success_and_typed_dependencies() {
        let res = client().get("/greet").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text(), "hello");
        assert!(res.header("x-request-id").is_some());
    }

    #[tokio::test]
    async fn unknown_route_and_method_use_the_error_envelope() {
        let client = client();
        let res = client.get("/nope").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let body: Value = res.json();
        assert_eq!(error_code(&body), "not_found");
        assert_eq!(
            body["error"]["request_id"].as_str(),
            res.header("x-request-id")
        );

        let res = client.post_json("/greet", &json!({})).await;
        assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(error_code(&res.json()), "method_not_allowed");
    }

    #[tokio::test]
    async fn body_rejections_are_consistent() {
        let client = client();
        let res = client
            .post_raw("/echo", "application/json", "{not json")
            .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error_code(&res.json()), "malformed_json");

        let res = client.post_raw("/echo", "text/plain", "{}").await;
        assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

        let big = format!("{{\"data\":\"{}\"}}", "x".repeat(200));
        let res = client.post_raw("/echo", "application/json", &big).await;
        assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(error_code(&res.json()), "payload_too_large");

        let res = client.post_json("/items", &json!({"name": 3})).await;
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error_code(&res.json()), "invalid_body");
    }

    #[tokio::test]
    async fn validation_failures_list_fields() {
        let res = client().post_json("/items", &json!({"name": "  "})).await;
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body: Value = res.json();
        assert_eq!(error_code(&body), "validation_failed");
        assert_eq!(body["error"]["details"]["fields"][0]["field"], "name");
        assert_eq!(body["error"]["details"]["fields"][0]["code"], "required");
    }

    #[tokio::test]
    async fn invalid_path_parameter_is_a_400() {
        let res = client().get("/items/abc").await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error_code(&res.json()), "invalid_path");
        assert_eq!(client().get("/items/7").await.json::<Value>()["id"], 7);
    }

    #[tokio::test]
    async fn internal_errors_hide_details() {
        let client = client();
        for uri in ["/fail", "/missing-dep"] {
            let res = client.get(uri).await;
            assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
            let text = res.text();
            assert!(text.contains("internal_error"));
            assert!(!text.contains("hunter2"));
            assert!(!text.contains("String"));
        }
    }

    #[tokio::test]
    async fn slow_requests_time_out() {
        let res = client().get("/slow").await;
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error_code(&res.json()), "timeout");
    }

    #[tokio::test]
    async fn request_ids_are_reused_only_when_safe() {
        let client = client();
        let res = client
            .get_with_header("/greet", "x-request-id", "abc-123")
            .await;
        assert_eq!(res.header("x-request-id"), Some("abc-123"));

        let res = client
            .get_with_header("/greet", "x-request-id", "evil\" injected <script>")
            .await;
        let id = res.header("x-request-id").unwrap();
        assert_ne!(id, "evil\" injected <script>");
        assert_eq!(id.len(), 36);
    }

    #[test]
    fn conflicting_route_sets_are_an_error_not_a_panic() {
        let a = Routes::new().route("/x", get(|| async { "a" }));
        let b = Routes::new().route("/x", get(|| async { "b" }));
        let err = build_router(ctx(), [a, b], &ServerConfig::default()).unwrap_err();
        assert!(matches!(err, WebError::RouteConflict(_)));
    }

    #[test]
    fn server_config_validation() {
        assert!(ServerConfig::default().validate().is_ok());
        let bad = ServerConfig {
            request_timeout_ms: 0,
            max_concurrent_requests: 0,
            ..ServerConfig::default()
        };
        assert_eq!(bad.validate().unwrap_err().len(), 2);
    }
}
