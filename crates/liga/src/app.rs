use std::fmt;
use std::path::Path;

use liga_core::{AppContext, ConfigError, ConfigLoader, Shutdown, StateError, StateMap};
use liga_modules::{
    Contributions, EventBus, LifecycleOptions, Module, ModuleSet, ModulesError, RunningModules,
};

#[cfg(feature = "web")]
use liga_web::{Routes, ServerConfig, WebError, axum::Router, testing::TestClient};

/// Anything that can stop an application from starting, serving or stopping cleanly.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Configuration could not be loaded or failed validation.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// Module resolution, registration, start or shutdown failed.
    #[error(transparent)]
    Modules(#[from] ModulesError),
    /// Shared state wiring failed.
    #[error(transparent)]
    State(#[from] StateError),
    /// The HTTP layer could not be assembled.
    #[cfg(feature = "web")]
    #[error(transparent)]
    Web(#[from] WebError),
    /// Binding or serving failed.
    #[error("server I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Entry point: `Application::builder()`.
#[derive(Debug, Clone, Copy)]
pub struct Application;

impl Application {
    /// Starts configuring an application.
    pub fn builder() -> ApplicationBuilder {
        ApplicationBuilder {
            name: "liga-app".to_owned(),
            loader: ConfigLoader::new(),
            modules: ModuleSet::new(),
            lifecycle: LifecycleOptions::default(),
            #[cfg(feature = "web")]
            routes: Vec::new(),
        }
    }
}

/// Collects configuration sources and modules. Nothing runs until [`ApplicationBuilder::build`].
pub struct ApplicationBuilder {
    name: String,
    loader: ConfigLoader,
    modules: ModuleSet,
    lifecycle: LifecycleOptions,
    #[cfg(feature = "web")]
    routes: Vec<Routes>,
}

impl fmt::Debug for ApplicationBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApplicationBuilder")
            .field("name", &self.name)
            .field("modules", &self.modules)
            .finish_non_exhaustive()
    }
}

impl ApplicationBuilder {
    /// Application name (logs, traces, [`AppContext::name`]).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Adds a configuration file that must exist (`.toml` or `.json`).
    pub fn config_file(mut self, path: impl AsRef<Path>) -> Self {
        self.loader = self.loader.file(path);
        self
    }

    /// Adds a configuration file that is skipped if missing.
    pub fn optional_config_file(mut self, path: impl AsRef<Path>) -> Self {
        self.loader = self.loader.optional_file(path);
        self
    }

    /// Reads `{PREFIX}__SECTION__KEY` environment variables (highest precedence).
    pub fn env_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.loader = self.loader.env_prefix(prefix);
        self
    }

    /// Uses these variables instead of the process environment (tests).
    pub fn env_vars<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.loader = self.loader.env_vars(vars);
        self
    }

    /// Replaces the configuration loader entirely.
    pub fn config_loader(mut self, loader: ConfigLoader) -> Self {
        self.loader = loader;
        self
    }

    /// Registers a module.
    pub fn module(mut self, module: impl Module) -> Self {
        self.modules.add(module);
        self
    }

    /// Time limits for module `start`/`shutdown` hooks.
    pub fn lifecycle(mut self, options: LifecycleOptions) -> Self {
        self.lifecycle = options;
        self
    }

    /// Adds routes that do not belong to a module.
    #[cfg(feature = "web")]
    pub fn routes(mut self, routes: Routes) -> Self {
        self.routes.push(routes);
        self
    }

    /// Loads and validates configuration, resolves and registers modules,
    /// starts them in dependency order and assembles the HTTP layer.
    ///
    /// Every configuration and wiring error is reported here, before any
    /// request is served. If something fails after modules started, they are
    /// shut down again before the error is returned.
    pub async fn build(self) -> Result<App, Error> {
        let config = self.loader.load()?;
        #[cfg(feature = "web")]
        let server: ServerConfig = config.section_validated("server")?;

        let resolved = self.modules.resolve()?;
        tracing::debug!(order = ?resolved.order(), "modules resolved");

        let events = EventBus::new();
        let mut state = StateMap::new();
        let mut contributions = Contributions::new();
        resolved.register(&config, &mut state, &mut contributions, &events)?;
        state.insert(events)?;

        let ctx = AppContext::new(self.name, config, state, Shutdown::new());
        let modules = resolved.start(&ctx, self.lifecycle).await?;

        #[cfg(feature = "web")]
        let router = {
            let mut routes: Vec<Routes> = contributions
                .take::<Routes>()
                .into_iter()
                .map(|(_, routes)| routes)
                .collect();
            routes.extend(self.routes);
            match liga_web::build_router(ctx.clone(), routes, &server) {
                Ok(router) => router,
                Err(err) => {
                    if let Err(shutdown) = modules.shutdown(&ctx).await {
                        tracing::error!(error = %shutdown, "shutdown after failed build was incomplete");
                    }
                    return Err(err.into());
                }
            }
        };
        #[cfg(not(feature = "web"))]
        drop(contributions);

        tracing::info!(app = ctx.name(), modules = ?modules.names(), "application ready");
        Ok(App {
            ctx,
            modules,
            #[cfg(feature = "web")]
            router,
            #[cfg(feature = "web")]
            server,
        })
    }
}

/// A started application. Call [`App::serve`], [`App::run_until_shutdown`]
/// or [`App::shutdown`] so module `shutdown` hooks run.
#[must_use = "an App holds started modules; serve it or shut it down"]
pub struct App {
    ctx: AppContext,
    modules: RunningModules,
    #[cfg(feature = "web")]
    router: Router,
    #[cfg(feature = "web")]
    server: ServerConfig,
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
            .field("name", &self.ctx.name())
            .field("modules", &self.modules)
            .finish_non_exhaustive()
    }
}

impl App {
    /// The shared context (configuration, state, shutdown signal).
    pub fn context(&self) -> &AppContext {
        &self.ctx
    }

    /// Started modules, in start order.
    pub fn module_names(&self) -> Vec<&'static str> {
        self.modules.names()
    }

    /// The validated `[server]` section.
    #[cfg(feature = "web")]
    pub fn server_config(&self) -> &ServerConfig {
        &self.server
    }

    /// The fully assembled router (with middleware and state).
    #[cfg(feature = "web")]
    pub fn router(&self) -> Router {
        self.router.clone()
    }

    /// An in-process client for tests.
    #[cfg(feature = "web")]
    pub fn test_client(&self) -> TestClient {
        TestClient::new(self.router.clone())
    }

    /// Binds `server.addr` and serves until Ctrl+C / SIGTERM or
    /// `context().shutdown().trigger()`, then drains requests (up to
    /// `server.shutdown_timeout_ms`) and shuts modules down.
    #[cfg(feature = "web")]
    pub async fn serve(self) -> Result<(), Error> {
        match tokio::net::TcpListener::bind(self.server.addr).await {
            Ok(listener) => self.serve_with_listener(listener).await,
            Err(err) => {
                tracing::error!(addr = %self.server.addr, error = %err, "could not bind");
                if let Err(shutdown) = self.modules.shutdown(&self.ctx).await {
                    tracing::error!(error = %shutdown, "shutdown after bind failure was incomplete");
                }
                Err(Error::Io(err))
            }
        }
    }

    /// Like [`App::serve`] on an existing listener (e.g. port 0 in tests).
    #[cfg(feature = "web")]
    pub async fn serve_with_listener(self, listener: tokio::net::TcpListener) -> Result<(), Error> {
        let shutdown = self.ctx.shutdown().clone();
        if let Ok(addr) = listener.local_addr() {
            tracing::info!(app = self.ctx.name(), %addr, "listening");
        }
        let signals = tokio::spawn(trigger_on_signal(shutdown.clone()));
        let mut server = tokio::spawn(liga_web::serve(
            listener,
            self.router.clone(),
            shutdown.clone(),
        ));

        let served = tokio::select! {
            result = &mut server => flatten(result),
            () = shutdown.wait() => {
                tracing::info!("shutdown requested; draining in-flight requests");
                match tokio::time::timeout(self.server.shutdown_timeout(), &mut server).await {
                    Ok(result) => flatten(result),
                    Err(_) => {
                        tracing::warn!("drain timed out; closing remaining connections");
                        server.abort();
                        Ok(())
                    }
                }
            }
        };
        signals.abort();
        shutdown.trigger();
        let modules = self.modules.shutdown(&self.ctx).await;
        tracing::info!("application stopped");
        served?;
        modules?;
        Ok(())
    }

    /// For applications without HTTP (workers): waits for Ctrl+C / SIGTERM or
    /// the shutdown signal, then shuts modules down.
    pub async fn run_until_shutdown(self) -> Result<(), Error> {
        let shutdown = self.ctx.shutdown().clone();
        let signals = tokio::spawn(trigger_on_signal(shutdown.clone()));
        shutdown.wait().await;
        signals.abort();
        self.shutdown().await
    }

    /// Triggers the shutdown signal and runs module `shutdown` hooks in reverse order.
    pub async fn shutdown(self) -> Result<(), Error> {
        self.ctx.shutdown().trigger();
        self.modules.shutdown(&self.ctx).await?;
        Ok(())
    }
}

#[cfg(feature = "web")]
fn flatten(result: Result<std::io::Result<()>, tokio::task::JoinError>) -> std::io::Result<()> {
    match result {
        Ok(inner) => inner,
        Err(join) => Err(std::io::Error::other(join.to_string())),
    }
}

async fn trigger_on_signal(shutdown: Shutdown) {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("received Ctrl+C"),
        () = terminate => tracing::info!("received SIGTERM"),
    }
    shutdown.trigger();
}
