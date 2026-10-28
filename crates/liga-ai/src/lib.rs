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

/// Framework-branded name for Agent.
pub type LigaAgent = Agent;
pub use message::{Message, Role, ToolCall, Usage};
pub use policy::Policy;
pub use provider::{ModelProvider, ModelRequest, ModelResponse, ProviderError, ProviderErrorKind};
pub use tool::{Permission, Tool, ToolContext, ToolError, ToolSpec};
