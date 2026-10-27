use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use liga_core::{AppContext, Config, StateMap};
use tracing::Instrument;

use crate::context::{Contributions, ModuleContext};
use crate::events::EventBus;
use crate::module::{DynModule, Module, ModuleError};

/// Errors produced while resolving, registering, starting or stopping modules.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ModulesError {
    /// Two modules share a name.
    #[error("module `{0}` is registered more than once")]
    Duplicate(&'static str),
    /// A declared dependency is not registered.
    #[error("module `{module}` depends on `{dependency}`, which is not registered")]
    MissingDependency {
        /// Module declaring the dependency.
        module: &'static str,
        /// Missing module name.
        dependency: String,
    },
    /// Dependencies form a cycle (the path starts and ends with the same module).
    #[error("dependency cycle: {}", .0.join(" -> "))]
    Cycle(Vec<&'static str>),
    /// A `register` hook failed.
    #[error("module `{module}` failed to register: {source}")]
    Register {
        /// Failing module.
        module: &'static str,
        /// Cause.
        #[source]
        source: ModuleError,
    },
    /// A `start` hook failed or timed out; already-started modules were shut down.
    #[error("module `{module}` failed to start: {source}")]
    Start {
        /// Failing module.
        module: &'static str,
        /// Cause.
        #[source]
        source: ModuleError,
    },
    /// One or more `shutdown` hooks failed or timed out (the others still ran).
    #[error("{} module(s) failed to shut down: {}", .0.len(), describe(.0))]
    Shutdown(Vec<(&'static str, ModuleError)>),
}

fn describe(failures: &[(&'static str, ModuleError)]) -> String {
    failures
        .iter()
        .map(|(module, err)| format!("{module} ({err})"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Time limits applied to lifecycle hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleOptions {
    /// Maximum time for each module's `start` hook. Default: 30 s.
    pub start_timeout: Duration,
    /// Maximum time for each module's `shutdown` hook. Default: 10 s.
    pub shutdown_timeout: Duration,
}

impl Default for LifecycleOptions {
    fn default() -> Self {
        Self {
            start_timeout: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(10),
        }
    }
}

/// Modules collected for an application, not yet validated.
#[derive(Default)]
pub struct ModuleSet {
    modules: Vec<Box<dyn DynModule>>,
}

impl fmt::Debug for ModuleSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.modules.iter().map(|m| m.name()))
            .finish()
    }
}

impl ModuleSet {
    /// Creates an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a module.
    pub fn add<M: Module>(&mut self, module: M) -> &mut Self {
        self.modules.push(Box::new(module));
        self
    }

    /// Adds a module (builder style).
    pub fn with<M: Module>(mut self, module: M) -> Self {
        self.add(module);
        self
    }

    /// Number of modules.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// `true` when no module was added.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Validates names and dependencies and orders modules so that every
    /// dependency comes before its dependents. Ties keep insertion order.
    pub fn resolve(self) -> Result<ResolvedModules, ModulesError> {
        let mut index: HashMap<&'static str, usize> = HashMap::new();
        for (i, module) in self.modules.iter().enumerate() {
            if index.insert(module.name(), i).is_some() {
                return Err(ModulesError::Duplicate(module.name()));
            }
        }
        for module in &self.modules {
            for dependency in module.dependencies() {
                if !index.contains_key(dependency) {
                    return Err(ModulesError::MissingDependency {
                        module: module.name(),
                        dependency: (*dependency).to_owned(),
                    });
                }
            }
        }

        let mut marks = vec![Mark::New; self.modules.len()];
        let mut stack = Vec::new();
        let mut order = Vec::with_capacity(self.modules.len());
        for i in 0..self.modules.len() {
            visit(i, &self.modules, &index, &mut marks, &mut stack, &mut order)
                .map_err(ModulesError::Cycle)?;
        }

        let mut slots: Vec<Option<Box<dyn DynModule>>> =
            self.modules.into_iter().map(Some).collect();
        let modules = order.into_iter().filter_map(|i| slots[i].take()).collect();
        Ok(ResolvedModules { modules })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    New,
    Visiting,
    Done,
}

fn visit(
    i: usize,
    modules: &[Box<dyn DynModule>],
    index: &HashMap<&'static str, usize>,
    marks: &mut [Mark],
    stack: &mut Vec<usize>,
    order: &mut Vec<usize>,
) -> Result<(), Vec<&'static str>> {
    match marks[i] {
        Mark::Done => return Ok(()),
        Mark::Visiting => {
            let start = stack.iter().position(|&s| s == i).unwrap_or(0);
            let mut path: Vec<&'static str> =
                stack[start..].iter().map(|&s| modules[s].name()).collect();
            path.push(modules[i].name());
            return Err(path);
        }
        Mark::New => {}
    }
    marks[i] = Mark::Visiting;
    stack.push(i);
    for dependency in modules[i].dependencies() {
        if let Some(&d) = index.get(dependency) {
            visit(d, modules, index, marks, stack, order)?;
        }
    }
    stack.pop();
    marks[i] = Mark::Done;
    order.push(i);
    Ok(())
}

/// Validated modules, in start order.
pub struct ResolvedModules {
    modules: Vec<Box<dyn DynModule>>,
}

impl fmt::Debug for ResolvedModules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.order()).finish()
    }
}

impl ResolvedModules {
    /// Module names in start order.
    pub fn order(&self) -> Vec<&'static str> {
        self.modules.iter().map(|m| m.name()).collect()
    }

    /// Runs every `register` hook in start order.
    pub fn register(
        &self,
        config: &Config,
        state: &mut StateMap,
        contributions: &mut Contributions,
        events: &EventBus,
    ) -> Result<(), ModulesError> {
        for module in &self.modules {
            let name = module.name();
            let mut ctx = ModuleContext::new(name, config, state, contributions, events);
            module
                .register(&mut ctx)
                .map_err(|source| ModulesError::Register {
                    module: name,
                    source,
                })?;
            tracing::debug!(module = name, "module registered");
        }
        Ok(())
    }

    /// Runs every `start` hook in order. On failure, shuts down the modules
    /// that already started (in reverse order) and returns the start error.
    /// The failing module's own `shutdown` is not called.
    pub async fn start(
        self,
        ctx: &AppContext,
        options: LifecycleOptions,
    ) -> Result<RunningModules, ModulesError> {
        let mut started: Vec<Box<dyn DynModule>> = Vec::with_capacity(self.modules.len());
        for module in self.modules {
            let name = module.name();
            let span = tracing::info_span!("module.start", module = name);
            let result = tokio::time::timeout(options.start_timeout, module.start(ctx))
                .instrument(span)
                .await;
            let error = match result {
                Ok(Ok(())) => {
                    tracing::info!(module = name, "module started");
                    started.push(module);
                    continue;
                }
                Ok(Err(err)) => err,
                Err(_) => {
                    ModuleError::new(format!("start timed out after {:?}", options.start_timeout))
                }
            };
            tracing::error!(module = name, error = %error, "module failed to start; rolling back");
            let rollback = RunningModules {
                modules: started,
                options,
            };
            if let Err(err) = rollback.shutdown(ctx).await {
                tracing::error!(error = %err, "rollback after failed start was incomplete");
            }
            return Err(ModulesError::Start {
                module: name,
                source: error,
            });
        }
        Ok(RunningModules {
            modules: started,
            options,
        })
    }
}

/// Started modules, awaiting shutdown.
#[must_use = "call `shutdown` to release module resources"]
pub struct RunningModules {
    modules: Vec<Box<dyn DynModule>>,
    options: LifecycleOptions,
}

impl fmt::Debug for RunningModules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.names()).finish()
    }
}

impl RunningModules {
    /// Names of started modules, in start order.
    pub fn names(&self) -> Vec<&'static str> {
        self.modules.iter().map(|m| m.name()).collect()
    }

    /// Runs every `shutdown` hook in reverse start order. A failing or
    /// timed-out hook does not prevent the remaining hooks from running.
    pub async fn shutdown(self, ctx: &AppContext) -> Result<(), ModulesError> {
        let mut failures = Vec::new();
        for module in self.modules.iter().rev() {
            let name = module.name();
            let span = tracing::info_span!("module.shutdown", module = name);
            match tokio::time::timeout(self.options.shutdown_timeout, module.shutdown(ctx))
                .instrument(span)
                .await
            {
                Ok(Ok(())) => tracing::info!(module = name, "module stopped"),
                Ok(Err(err)) => {
                    tracing::error!(module = name, error = %err, "module failed to shut down");
                    failures.push((name, err));
                }
                Err(_) => {
                    let err = ModuleError::new(format!(
                        "shutdown timed out after {:?}",
                        self.options.shutdown_timeout
                    ));
                    tracing::error!(module = name, error = %err, "module failed to shut down");
                    failures.push((name, err));
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(ModulesError::Shutdown(failures))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use liga_core::Shutdown;
    use std::sync::{Arc, Mutex};

    type Log = Arc<Mutex<Vec<String>>>;

    struct TestModule {
        name: &'static str,
        deps: &'static [&'static str],
        log: Log,
        fail_start: bool,
        fail_shutdown: bool,
    }

    impl TestModule {
        fn new(name: &'static str, deps: &'static [&'static str], log: &Log) -> Self {
            Self {
                name,
                deps,
                log: log.clone(),
                fail_start: false,
                fail_shutdown: false,
            }
        }
    }

    impl Module for TestModule {
        fn name(&self) -> &'static str {
            self.name
        }

        fn dependencies(&self) -> &'static [&'static str] {
            self.deps
        }

        fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("register:{}", self.name));
            ctx.contribute(self.name.to_owned());
            Ok(())
        }

        async fn start(&self, _ctx: &AppContext) -> Result<(), ModuleError> {
            if self.fail_start {
                return Err(ModuleError::new("cannot connect"));
            }
            self.log
                .lock()
                .unwrap()
                .push(format!("start:{}", self.name));
            Ok(())
        }

        async fn shutdown(&self, _ctx: &AppContext) -> Result<(), ModuleError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("shutdown:{}", self.name));
            if self.fail_shutdown {
                return Err(ModuleError::new("flush failed"));
            }
            Ok(())
        }
    }

    fn ctx() -> AppContext {
        AppContext::new("test", Config::default(), StateMap::new(), Shutdown::new())
    }

    fn log() -> Log {
        Arc::new(Mutex::new(Vec::new()))
    }

    #[test]
    fn orders_dependencies_first_and_keeps_insertion_order_for_ties() {
        let log = log();
        let resolved = ModuleSet::new()
            .with(TestModule::new("api", &["db", "cache"], &log))
            .with(TestModule::new("metrics", &[], &log))
            .with(TestModule::new("cache", &["db"], &log))
            .with(TestModule::new("db", &[], &log))
            .resolve()
            .unwrap();
        assert_eq!(resolved.order(), ["db", "cache", "api", "metrics"]);
    }

    #[test]
    fn rejects_duplicates_missing_dependencies_and_cycles() {
        let log = log();
        let dup = ModuleSet::new()
            .with(TestModule::new("a", &[], &log))
            .with(TestModule::new("a", &[], &log))
            .resolve();
        assert!(matches!(dup, Err(ModulesError::Duplicate("a"))));

        let missing = ModuleSet::new()
            .with(TestModule::new("a", &["ghost"], &log))
            .resolve();
        assert!(matches!(
            missing,
            Err(ModulesError::MissingDependency { module: "a", .. })
        ));

        let cycle = ModuleSet::new()
            .with(TestModule::new("a", &["b"], &log))
            .with(TestModule::new("b", &["c"], &log))
            .with(TestModule::new("c", &["a"], &log))
            .resolve()
            .unwrap_err();
        assert_eq!(cycle.to_string(), "dependency cycle: a -> b -> c -> a");

        let self_cycle = ModuleSet::new()
            .with(TestModule::new("a", &["a"], &log))
            .resolve();
        assert!(matches!(self_cycle, Err(ModulesError::Cycle(path)) if path == ["a", "a"]));
    }

    #[tokio::test]
    async fn starts_in_order_and_shuts_down_in_reverse() {
        let log = log();
        let resolved = ModuleSet::new()
            .with(TestModule::new("web", &["db"], &log))
            .with(TestModule::new("db", &[], &log))
            .resolve()
            .unwrap();
        let mut state = StateMap::new();
        let mut contributions = Contributions::new();
        resolved
            .register(
                &Config::default(),
                &mut state,
                &mut contributions,
                &EventBus::new(),
            )
            .unwrap();
        let contributed: Vec<_> = contributions
            .take::<String>()
            .into_iter()
            .map(|(m, _)| m)
            .collect();
        assert_eq!(contributed, ["db", "web"]);

        let ctx = ctx();
        let running = resolved
            .start(&ctx, LifecycleOptions::default())
            .await
            .unwrap();
        running.shutdown(&ctx).await.unwrap();
        assert_eq!(
            *log.lock().unwrap(),
            [
                "register:db",
                "register:web",
                "start:db",
                "start:web",
                "shutdown:web",
                "shutdown:db"
            ]
        );
    }

    #[tokio::test]
    async fn failed_start_rolls_back_started_modules_only() {
        let log = log();
        let mut failing = TestModule::new("api", &["db"], &log);
        failing.fail_start = true;
        let resolved = ModuleSet::new()
            .with(TestModule::new("db", &[], &log))
            .with(failing)
            .with(TestModule::new("worker", &["api"], &log))
            .resolve()
            .unwrap();
        let err = resolved
            .start(&ctx(), LifecycleOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, ModulesError::Start { module: "api", .. }));
        assert_eq!(*log.lock().unwrap(), ["start:db", "shutdown:db"]);
    }

    #[tokio::test]
    async fn shutdown_continues_after_failures_and_reports_them() {
        let log = log();
        let mut flaky = TestModule::new("b", &["a"], &log);
        flaky.fail_shutdown = true;
        let ctx = ctx();
        let running = ModuleSet::new()
            .with(TestModule::new("a", &[], &log))
            .with(flaky)
            .resolve()
            .unwrap()
            .start(&ctx, LifecycleOptions::default())
            .await
            .unwrap();
        let err = running.shutdown(&ctx).await.unwrap_err();
        assert_eq!(
            err.to_string(),
            "1 module(s) failed to shut down: b (flush failed)"
        );
        assert!(log.lock().unwrap().contains(&"shutdown:a".to_owned()));
    }

    #[tokio::test]
    async fn start_timeout_is_enforced() {
        struct Slow;
        impl Module for Slow {
            fn name(&self) -> &'static str {
                "slow"
            }
            async fn start(&self, _ctx: &AppContext) -> Result<(), ModuleError> {
                tokio::time::sleep(Duration::from_secs(10)).await;
                Ok(())
            }
        }
        let options = LifecycleOptions {
            start_timeout: Duration::from_millis(20),
            ..LifecycleOptions::default()
        };
        let err = ModuleSet::new()
            .with(Slow)
            .resolve()
            .unwrap()
            .start(&ctx(), options)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn register_sees_state_provided_by_dependencies() {
        struct Db;
        impl Module for Db {
            fn name(&self) -> &'static str {
                "db"
            }
            fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
                ctx.provide(42_u32)
            }
        }
        struct Repo;
        impl Module for Repo {
            fn name(&self) -> &'static str {
                "repo"
            }
            fn dependencies(&self) -> &'static [&'static str] {
                &["db"]
            }
            fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
                let pool = ctx
                    .get::<u32>()
                    .ok_or_else(|| ModuleError::new("db pool missing"))?;
                ctx.provide(format!("repo using pool {pool}"))
            }
        }
        let resolved = ModuleSet::new().with(Repo).with(Db).resolve().unwrap();
        let mut state = StateMap::new();
        resolved
            .register(
                &Config::default(),
                &mut state,
                &mut Contributions::new(),
                &EventBus::new(),
            )
            .unwrap();
        assert_eq!(*state.get::<String>().unwrap(), "repo using pool 42");
    }

    #[test]
    fn duplicate_state_fails_registration() {
        struct Provider(&'static str);
        impl Module for Provider {
            fn name(&self) -> &'static str {
                self.0
            }
            fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
                ctx.provide(1_u8)
            }
        }
        let resolved = ModuleSet::new()
            .with(Provider("a"))
            .with(Provider("b"))
            .resolve()
            .unwrap();
        let err = resolved
            .register(
                &Config::default(),
                &mut StateMap::new(),
                &mut Contributions::new(),
                &EventBus::new(),
            )
            .unwrap_err();
        assert!(matches!(err, ModulesError::Register { module: "b", .. }));
    }
}
