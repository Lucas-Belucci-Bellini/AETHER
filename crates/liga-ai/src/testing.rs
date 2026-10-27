//! Deterministic test doubles. No network, no API keys.

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::provider::{
    ModelProvider, ModelRequest, ModelResponse, ProviderError, ProviderErrorKind,
};

/// A provider that replays a fixed script of responses and records every
/// request it receives. Returns an error once the script is exhausted.
#[derive(Debug, Default)]
pub struct ScriptedProvider {
    script: Mutex<VecDeque<Result<ModelResponse, ProviderError>>>,
    requests: Mutex<Vec<ModelRequest>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl ScriptedProvider {
    /// A provider that answers with `responses`, in order.
    pub fn new(responses: impl IntoIterator<Item = ModelResponse>) -> Self {
        Self {
            script: Mutex::new(responses.into_iter().map(Ok).collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    /// A provider with an empty script.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Appends a successful response.
    pub fn push(self, response: ModelResponse) -> Self {
        lock(&self.script).push_back(Ok(response));
        self
    }

    /// Appends a failure.
    pub fn push_error(self, error: ProviderError) -> Self {
        lock(&self.script).push_back(Err(error));
        self
    }

    /// Every request received so far.
    pub fn requests(&self) -> Vec<ModelRequest> {
        lock(&self.requests).clone()
    }
}

impl ModelProvider for ScriptedProvider {
    fn name(&self) -> &str {
        "scripted"
    }

    async fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        lock(&self.requests).push(request.clone());
        lock(&self.script).pop_front().unwrap_or_else(|| {
            Err(ProviderError::new(
                ProviderErrorKind::Other,
                "scripted provider has no more responses",
            ))
        })
    }
}
