//! Liga AI building blocks for agents that call tools safely.
//! LigaAgent is the framework-branded alias; Agent remains supported.

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
