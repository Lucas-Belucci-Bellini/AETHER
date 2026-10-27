use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use liga_core::{Config, StateMap, Validate};
use serde::de::DeserializeOwned;

use crate::events::EventBus;
use crate::module::ModuleError;

/// Multi-valued, type-indexed values that modules hand to integrations.
///
/// For example, each module may contribute a `liga_web::Routes`; the web
/// integration takes all of them after registration.
#[derive(Default)]
pub struct Contributions {
    entries: HashMap<TypeId, Vec<Contribution>>,
}

/// A contributed value and the module that contributed it.
type Contribution = (&'static str, Box<dyn Any + Send>);

impl fmt::Debug for Contributions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count: usize = self.entries.values().map(Vec::len).sum();
        f.debug_struct("Contributions")
            .field("count", &count)
            .finish()
    }
}

impl Contributions {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a value contributed by `module`.
    pub fn push<T: Send + 'static>(&mut self, module: &'static str, value: T) {
        self.entries
            .entry(TypeId::of::<T>())
            .or_default()
            .push((module, Box::new(value)));
    }

    /// Removes and returns every value of type `T`, in registration order,
    /// paired with the name of the module that contributed it.
    pub fn take<T: Send + 'static>(&mut self) -> Vec<(&'static str, T)> {
        self.entries
            .remove(&TypeId::of::<T>())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(module, value)| value.downcast::<T>().ok().map(|v| (module, *v)))
            .collect()
    }
}

/// What a module can touch during [`Module::register`](crate::Module::register).
pub struct ModuleContext<'a> {
    pub(crate) module: &'static str,
    pub(crate) config: &'a Config,
    pub(crate) state: &'a mut StateMap,
    pub(crate) contributions: &'a mut Contributions,
    pub(crate) events: &'a EventBus,
}

impl fmt::Debug for ModuleContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModuleContext")
            .field("module", &self.module)
            .finish_non_exhaustive()
    }
}

impl<'a> ModuleContext<'a> {
    /// Builds a context by hand (mostly useful for testing a module in isolation).
    pub fn new(
        module: &'static str,
        config: &'a Config,
        state: &'a mut StateMap,
        contributions: &'a mut Contributions,
        events: &'a EventBus,
    ) -> Self {
        Self {
            module,
            config,
            state,
            contributions,
            events,
        }
    }

    /// Name of the module being registered.
    pub fn module_name(&self) -> &'static str {
        self.module
    }

    /// The merged application configuration.
    pub fn config(&self) -> &Config {
        self.config
    }

    /// Reads and validates a configuration section (missing section = empty table).
    pub fn config_section<T: DeserializeOwned + Validate>(
        &self,
        path: &str,
    ) -> Result<T, ModuleError> {
        Ok(self.config.section_validated(path)?)
    }

    /// Makes `value` available to every module, handler and task as shared state.
    pub fn provide<T: Send + Sync + 'static>(&mut self, value: T) -> Result<(), ModuleError> {
        Ok(self.state.insert(value)?)
    }

    /// Like [`ModuleContext::provide`] for a value that is already shared.
    pub fn provide_arc<T: Send + Sync + 'static>(
        &mut self,
        value: Arc<T>,
    ) -> Result<(), ModuleError> {
        Ok(self.state.insert_arc(value)?)
    }

    /// Reads shared state provided by a module registered earlier
    /// (dependencies always register first).
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.state.get::<T>()
    }

    /// Hands a value to integrations (e.g. HTTP routes to `liga-web`).
    pub fn contribute<T: Send + 'static>(&mut self, value: T) {
        self.contributions.push(self.module, value);
    }

    /// The application event bus.
    pub fn events(&self) -> &EventBus {
        self.events
    }
}
