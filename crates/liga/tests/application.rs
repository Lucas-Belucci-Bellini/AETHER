//! End-to-end tests of the application builder and lifecycle.
#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use liga::prelude::*;
use liga::web::testing::TestClient;
use serde::Deserialize;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Log = Arc<Mutex<Vec<String>>>;

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct GreeterConfig {
    greeting: String,
}

impl Default for GreeterConfig {
    fn default() -> Self {
        Self {
            greeting: "hello".into(),
        }
    }
}

impl Validate for GreeterConfig {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors.check(
            !self.greeting.is_empty(),
            "greeting",
            "required",
            "must not be empty",
        );
        errors.into_result()
    }
}

struct Greeter {
    log: Log,
    fail_start: bool,
}

impl Greeter {
    fn new(log: &Log) -> Self {
        Self {
            log: log.clone(),
            fail_start: false,
        }
    }
}

impl Module for Greeter {
    fn name(&self) -> &'static str {
        "greeter"
    }

    fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
        let config: GreeterConfig = ctx.config_section("greeter")?;
        ctx.provide(config)?;
        ctx.contribute(Routes::new().route(
            "/greet",
            get(|Dep(config): Dep<GreeterConfig>| async move { config.greeting.clone() }),
        ));
        Ok(())
    }

    async fn start(&self, _ctx: &AppContext) -> Result<(), ModuleError> {
        if self.fail_start {
            return Err(ModuleError::new("greeter could not start"));
        }
        self.log.lock().unwrap().push("start:greeter".into());
        Ok(())
    }

    async fn shutdown(&self, _ctx: &AppContext) -> Result<(), ModuleError> {
        self.log.lock().unwrap().push("shutdown:greeter".into());
        Ok(())
    }
}

struct Echo {
    log: Log,
}

impl Module for Echo {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["greeter"]
    }

    fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
        // Same path and method as the greeter: a real conflict.
        ctx.contribute(Routes::new().route("/greet", get(|| async { "conflict" })));
        Ok(())
    }

    async fn start(&self, _ctx: &AppContext) -> Result<(), ModuleError> {
        self.log.lock().unwrap().push("start:echo".into());
        Ok(())
    }

    async fn shutdown(&self, _ctx: &AppContext) -> Result<(), ModuleError> {
        self.log.lock().unwrap().push("shutdown:echo".into());
        Ok(())
    }
}

fn log() -> Log {
    Arc::new(Mutex::new(Vec::new()))
}

fn builder(log: &Log) -> ApplicationBuilder {
    Application::builder()
        .name("test-app")
        .env_prefix("TEST")
        .env_vars([("TEST__GREETER__GREETING", "olá")])
        .module(Greeter::new(log))
}

#[tokio::test]
async fn builds_serves_in_process_and_shuts_down() {
    let log = log();
    let app = builder(&log).build().await.unwrap();
    assert_eq!(app.module_names(), ["greeter"]);
    let client: TestClient = app.test_client();
    let res = client.get("/greet").await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text(), "olá");
    assert!(app.context().require::<EventBus>().is_ok());
    app.shutdown().await.unwrap();
    assert_eq!(*log.lock().unwrap(), ["start:greeter", "shutdown:greeter"]);
}

#[tokio::test]
async fn invalid_configuration_fails_before_any_module_starts() {
    let log = log();
    let err = builder(&log)
        .env_vars([("TEST__SERVER__REQUEST_TIMEOUT_MS", "0")])
        .build()
        .await
        .unwrap_err();
    assert!(matches!(err, liga::Error::Config(_)), "{err}");
    assert!(err.to_string().contains("request_timeout_ms"));

    let err = builder(&log)
        .env_vars([("TEST__GREETER__GREETING", "")])
        .build()
        .await
        .unwrap_err();
    assert!(err.to_string().contains("greeting"), "{err}");
    assert!(log.lock().unwrap().is_empty());
}

#[tokio::test]
async fn failed_start_is_reported() {
    let log = log();
    let mut greeter = Greeter::new(&log);
    greeter.fail_start = true;
    let err = Application::builder()
        .module(greeter)
        .build()
        .await
        .unwrap_err();
    assert!(err.to_string().contains("greeter could not start"));
}

#[tokio::test]
async fn route_conflicts_shut_started_modules_down() {
    let log = log();
    let err = builder(&log)
        .module(Echo { log: log.clone() })
        .build()
        .await
        .unwrap_err();
    assert!(matches!(err, liga::Error::Web(_)), "{err}");
    assert_eq!(
        *log.lock().unwrap(),
        [
            "start:greeter",
            "start:echo",
            "shutdown:echo",
            "shutdown:greeter"
        ]
    );
}

#[tokio::test]
async fn serves_real_http_and_stops_on_shutdown_signal() {
    let log = log();
    let app = builder(&log).build().await.unwrap();
    let shutdown = app.context().shutdown().clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(app.serve_with_listener(listener));

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(b"GET /missing HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).await.unwrap();
    assert!(raw.starts_with("HTTP/1.1 404"), "{raw}");
    let body: Value = serde_json::from_str(raw.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["error"]["code"], "not_found");

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(log.lock().unwrap().last().unwrap(), "shutdown:greeter");
}
