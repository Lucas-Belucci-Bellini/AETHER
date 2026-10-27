use std::borrow::Cow;
use std::error::Error as StdError;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::message::{Message, ToolCall, Usage};
use crate::tool::ToolSpec;

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Input for one model call.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelRequest {
    /// Conversation so far.
    pub messages: Vec<Message>,
    /// Tools the model may request.
    pub tools: Vec<ToolSpec>,
    /// Upper bound for generated tokens, if the provider supports it.
    pub max_output_tokens: Option<u32>,
}

/// Output of one model call: text, tool calls, or both.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelResponse {
    /// Generated text (the final answer when there are no tool calls).
    pub content: String,
    /// Tools the model wants to call.
    pub tool_calls: Vec<ToolCall>,
    /// Token usage, if reported.
    pub usage: Option<Usage>,
}

impl ModelResponse {
    /// A plain text answer.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Self::default()
        }
    }

    /// A response requesting one tool call.
    pub fn tool_call(id: impl Into<String>, name: impl Into<String>, arguments: Value) -> Self {
        Self::default().with_tool_call(id, name, arguments)
    }

    /// Adds a tool call.
    pub fn with_tool_call(
        mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: Value,
    ) -> Self {
        self.tool_calls.push(ToolCall {
            id: id.into(),
            name: name.into(),
            arguments,
        });
        self
    }

    /// Sets usage.
    pub fn with_usage(mut self, input_tokens: u64, output_tokens: u64) -> Self {
        self.usage = Some(Usage {
            input_tokens,
            output_tokens,
        });
        self
    }
}

/// Category of a provider failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderErrorKind {
    /// Too many requests; see [`ProviderError::retry_after`].
    RateLimited,
    /// The call took too long.
    Timeout,
    /// Provider temporarily unavailable (5xx, connection error).
    Unavailable,
    /// Credentials missing or rejected.
    Authentication,
    /// The request was rejected as invalid.
    InvalidRequest,
    /// Anything else.
    Other,
}

/// Error from a [`ModelProvider`]. Messages must not contain credentials.
#[derive(Debug, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct ProviderError {
    kind: ProviderErrorKind,
    message: Cow<'static, str>,
    retry_after: Option<Duration>,
    #[source]
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl ProviderError {
    /// Creates an error.
    pub fn new(kind: ProviderErrorKind, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind,
            message: message.into(),
            retry_after: None,
            source: None,
        }
    }

    /// Suggested wait before retrying.
    pub fn with_retry_after(mut self, retry_after: Duration) -> Self {
        self.retry_after = Some(retry_after);
        self
    }

    /// Attaches a cause.
    pub fn with_source(mut self, source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Category.
    pub fn kind(&self) -> ProviderErrorKind {
        self.kind
    }

    /// Suggested wait before retrying, if any.
    pub fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    /// `true` for rate limits, timeouts and unavailability.
    pub fn is_transient(&self) -> bool {
        matches!(
            self.kind,
            ProviderErrorKind::RateLimited
                | ProviderErrorKind::Timeout
                | ProviderErrorKind::Unavailable
        )
    }
}

/// A language-model backend.
///
/// Implementations should enforce their own network timeouts and read
/// credentials from the environment or a secret manager, never from code.
pub trait ModelProvider: Send + Sync + 'static {
    /// Short provider name for traces (e.g. `"openai"`).
    fn name(&self) -> &str;

    /// Performs one completion.
    fn complete(
        &self,
        request: &ModelRequest,
    ) -> impl Future<Output = Result<ModelResponse, ProviderError>> + Send;
}

impl<P: ModelProvider> ModelProvider for Arc<P> {
    fn name(&self) -> &str {
        (**self).name()
    }

    fn complete(
        &self,
        request: &ModelRequest,
    ) -> impl Future<Output = Result<ModelResponse, ProviderError>> + Send {
        (**self).complete(request)
    }
}

pub(crate) trait DynProvider: Send + Sync {
    fn name(&self) -> &str;
    fn complete<'a>(
        &'a self,
        request: &'a ModelRequest,
    ) -> BoxFuture<'a, Result<ModelResponse, ProviderError>>;
}

impl<P: ModelProvider> DynProvider for P {
    fn name(&self) -> &str {
        ModelProvider::name(self)
    }

    fn complete<'a>(
        &'a self,
        request: &'a ModelRequest,
    ) -> BoxFuture<'a, Result<ModelResponse, ProviderError>> {
        Box::pin(ModelProvider::complete(self, request))
    }
}
