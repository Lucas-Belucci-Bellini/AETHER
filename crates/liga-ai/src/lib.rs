//! # liga-ai
//!
//! Provider-neutral building blocks for agents that call tools **safely**.
//!
//! - [`ModelProvider`]: one async method; adapters for real APIs live in
//!   separate crates. [`testing::ScriptedProvider`] makes tests deterministic.
//! - [`Tool`]: typed arguments (validated by deserialization before the call),
//!   typed output and the [`Permission`]s it requires.
//! - [`Policy`]: deny-by-default grants. Model output never grants anything.
//! - [`Agent`]: a loop bounded by steps, total time, per-tool time, number of
//!   tool calls and argument/output sizes.
//!
//! What the agent sends back to the model when a call is refused (unknown
//! tool, missing permission, invalid or oversized arguments, timeout) is a
//! JSON error string, so the model can recover without the tool ever running.
//!
//! Prompt and tool contents are never logged; spans carry IDs, names,
//! outcomes and sizes only.

mod agent;
mod message;
mod policy;
mod provider;
pub mod testing;
mod tool;

pub use agent::{
    Agent, AgentBuildError, AgentBuilder, AgentError, AgentOutcome, Limits, ToolCallOutcome,
    ToolCallRecord,
};

/// Official Aether name for Agent.
pub type AetherAgent = Agent;
/// Official Aether name for AgentBuilder.
pub type AetherAgentBuilder = AgentBuilder;
/// Official Aether name for AgentError.
pub type AetherAgentError = AgentError;
/// Official Aether name for AgentOutcome.
pub type AetherAgentOutcome = AgentOutcome;
/// Official Aether name for errors building an agent.
pub type AetherAgentBuildError = AgentBuildError;
/// Official Aether name for agent execution limits.
pub type AetherLimits = Limits;
/// Official Aether name for a tool call's outcome.
pub type AetherToolCallOutcome = ToolCallOutcome;
/// Official Aether name for a recorded tool call.
pub type AetherToolCallRecord = ToolCallRecord;

/// Compatibility alias for the previous public branding.
pub type LigaAgent = AetherAgent;

pub use message::{Message, Role, ToolCall, Usage};
/// Official Aether name for a model message.
pub use message::Message as AetherMessage;
/// Official Aether name for a conversation role.
pub use message::Role as AetherRole;
/// Official Aether name for a model tool call.
pub use message::ToolCall as AetherToolCall;
/// Official Aether name for model usage statistics.
pub use message::Usage as AetherUsage;
pub use policy::Policy;
/// Official Aether name for an agent permission policy.
pub use policy::Policy as AetherPolicy;
pub use provider::{ModelProvider, ModelRequest, ModelResponse, ProviderError, ProviderErrorKind};
/// Official Aether name for model provider adapters.
pub use provider::ModelProvider as AetherModelProvider;
/// Official Aether name for a model request.
pub use provider::ModelRequest as AetherModelRequest;
/// Official Aether name for a model response.
pub use provider::ModelResponse as AetherModelResponse;
/// Official Aether name for provider errors.
pub use provider::ProviderError as AetherProviderError;
/// Official Aether name for provider error categories.
pub use provider::ProviderErrorKind as AetherProviderErrorKind;
pub use tool::{Permission, Tool, ToolContext, ToolError, ToolSpec};
/// Official Aether name for tool permission declarations.
pub use tool::Permission as AetherPermission;
/// Official Aether name for a callable tool.
pub use tool::Tool as AetherTool;
/// Official Aether name for a tool execution context.
pub use tool::ToolContext as AetherToolContext;
/// Official Aether name for tool execution errors.
pub use tool::ToolError as AetherToolError;
/// Official Aether name for a tool specification.
pub use tool::ToolSpec as AetherToolSpec;
