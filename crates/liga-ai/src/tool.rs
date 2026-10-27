use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::provider::BoxFuture;

/// A capability a tool needs, e.g. `"project.read"` or `"email.send"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Permission(Cow<'static, str>);

impl Permission {
    /// Creates a permission.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self(name.into())
    }

    /// Permission name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&'static str> for Permission {
    fn from(name: &'static str) -> Self {
        Self::new(name)
    }
}

impl From<String> for Permission {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

/// Everything the model and the policy need to know about a tool.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    /// Unique name, `[A-Za-z0-9_-]{1,64}`.
    pub name: String,
    /// What the tool does (shown to the model).
    pub description: String,
    /// JSON Schema of the arguments (shown to the model). The authoritative
    /// validation is deserialization into [`Tool::Args`].
    pub parameters: Value,
    /// Permissions the policy must grant. At least one is required.
    pub permissions: Vec<Permission>,
}

impl ToolSpec {
    /// Starts a spec with an empty-object parameter schema.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
            permissions: Vec::new(),
        }
    }

    /// Sets the JSON Schema of the arguments.
    pub fn parameters(mut self, schema: Value) -> Self {
        self.parameters = schema;
        self
    }

    /// Adds a required permission.
    pub fn permission(mut self, permission: impl Into<Permission>) -> Self {
        self.permissions.push(permission.into());
        self
    }
}

/// Information about the current call, passed to [`Tool::call`].
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub(crate) run_id: Arc<str>,
    pub(crate) step: u32,
    pub(crate) call_id: String,
}

impl ToolContext {
    /// ID of the agent run.
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// 1-based agent step.
    pub fn step(&self) -> u32 {
        self.step
    }

    /// Provider call ID.
    pub fn call_id(&self) -> &str {
        &self.call_id
    }
}

/// Error from a tool. The message is sent back to the model, so keep it
/// free of secrets and internal details.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ToolError {
    message: Cow<'static, str>,
    #[source]
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl ToolError {
    /// Creates an error with a model-safe message.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// Attaches an internal cause (logged, not sent to the model).
    pub fn with_source(mut self, source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        self.source = Some(source.into());
        self
    }
}

/// A function the model may ask the agent to run.
///
/// Arguments arrive as JSON and are deserialized into [`Tool::Args`] before
/// `call` runs; use `#[serde(deny_unknown_fields)]` for strict validation and
/// check value ranges inside `call`.
pub trait Tool: Send + Sync + 'static {
    /// Typed arguments.
    type Args: DeserializeOwned + Send;
    /// Typed result, serialized to JSON for the model.
    type Output: Serialize + Send;

    /// Name, description, schema and required permissions.
    fn spec(&self) -> ToolSpec;

    /// Runs the tool.
    fn call(
        &self,
        args: Self::Args,
        ctx: &ToolContext,
    ) -> impl Future<Output = Result<Self::Output, ToolError>> + Send;
}

#[derive(Debug)]
pub(crate) enum InvokeError {
    InvalidArguments(String),
    Failed(ToolError),
    Output(String),
}

pub(crate) trait DynTool: Send + Sync {
    fn call_json<'a>(
        &'a self,
        args: Value,
        ctx: &'a ToolContext,
    ) -> BoxFuture<'a, Result<Value, InvokeError>>;
}

impl<T: Tool> DynTool for T {
    fn call_json<'a>(
        &'a self,
        args: Value,
        ctx: &'a ToolContext,
    ) -> BoxFuture<'a, Result<Value, InvokeError>> {
        Box::pin(async move {
            let args: T::Args = serde_json::from_value(args)
                .map_err(|e| InvokeError::InvalidArguments(e.to_string()))?;
            let output = self.call(args, ctx).await.map_err(InvokeError::Failed)?;
            serde_json::to_value(output).map_err(|e| InvokeError::Output(e.to_string()))
        })
    }
}
