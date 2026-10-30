use liga_core::{AppContext, Config, Shutdown, StateMap};
use liga_modules::{LifecycleOptions, Module, ModuleError, ModuleSet, ModulesError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Log = Arc<Mutex<Vec<&'static str>>>;

struct HookModule {
    name: &'static str,
    fails_to_start: bool,
    shutdown_delay: Duration,
    log: Log,
}

impl HookModule {
    fn new(name: &'static str, log: &Log) -> Self {
        Self {
            name,
            fails_to_start: false,
            shutdown_delay: Duration::ZERO,
            log: Arc::clone(log),
        }
    }
}

impl Module for HookModule {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn start(&self, _ctx: &AppContext) -> Result<(), ModuleError> {
        self.log.lock().unwrap().push(self.name);
        if self.fails_to_start {
            return Err(ModuleError::new("deliberate start failure"));
        }
        Ok(())
    }

    async fn shutdown(&self, _ctx: &AppContext) -> Result<(), ModuleError> {
        self.log.lock().unwrap().push(self.name);
        tokio::time::sleep(self.shutdown_delay).await;
        Ok(())
    }
}

fn app_context() -> AppContext {
    AppContext::new("lifecycle-contract-test", Config::default(), StateMap::new(), Shutdown::new())
}

fn log() -> Log {
    Arc::new(Mutex::new(Vec::new()))
}

#[tokio::test]
async fn public_lifecycle_api_rolls_back_only_started_modules() {
    let events = log();
    let mut failing = HookModule::new("failing", &events);
    failing.fails_to_start = true;

    let resolved = ModuleSet::new()
        .with(HookModule::new("database", &events))
        .with(failing)
        .with(HookModule::new("worker", &events))
        .resolve()
        .unwrap();

    let error = resolved
        .start(&app_context(), LifecycleOptions::default())
        .await
        .unwrap_err();

    assert!(matches!(error, ModulesError::Start { module: "failing", .. }));
    assert_eq!(*events.lock().unwrap(), ["database", "failing", "database"]);
}

#[tokio::test]
async fn shutdown_timeout_is_reported_without_skipping_remaining_hooks() {
    let events = log();
    let mut slow = HookModule::new("slow", &events);
    slow.shutdown_delay = Duration::from_millis(100);

    let running = ModuleSet::new()
        .with(slow)
        .with(HookModule::new("fast", &events))
        .resolve()
        .unwrap()
        .start(&app_context(), LifecycleOptions::default())
        .await
        .unwrap();

    let options = LifecycleOptions {
        shutdown_timeout: Duration::from_millis(10),
        ..LifecycleOptions::default()
    };
    // Rebuild with the short timeout: options are fixed when start creates RunningModules.
    drop(running);

    let running = ModuleSet::new()
        .with({
            let mut slow = HookModule::new("slow", &events);
            slow.shutdown_delay = Duration::from_millis(100);
            slow
        })
        .with(HookModule::new("fast", &events))
        .resolve()
        .unwrap()
        .start(&app_context(), options)
        .await
        .unwrap();

    let error = running.shutdown(&app_context()).await.unwrap_err();
    assert!(matches!(error, ModulesError::Shutdown(failures) if failures.len() == 1 && failures[0].0 == "slow"));
    assert!(events.lock().unwrap().ends_with(&["fast", "slow"]));
}
