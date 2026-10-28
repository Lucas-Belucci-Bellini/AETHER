//! `POST /assistant/ask` answers questions with an AETHER [`AetherAgent`] that can use
//! two tools:
//!
//! | tool                  | permission      | granted by default |
//! |-----------------------|-----------------|--------------------|
//! | `read_project_status` | `project.read`  | yes                |
//! | `create_ticket`       | `tickets.write` | no (`[assistant] allow_ticket_creation = true`) |
//!
//! The model is [`KeywordModel`], a deterministic stand-in so the example runs
//! without network access or API keys. Replace it with any
//! [`ModelProvider`] adapter; nothing else changes.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use liga::ai::{
    AetherAgent, AetherAgentError, Message, ModelRequest, ModelResponse, ProviderError, Role, ToolCallOutcome,
};
use liga::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

// ------------------------------------------------------------------ model

/// Picks a tool from keywords in the user's message and phrases the answer
/// from the tool result. Stands in for a real model.
#[derive(Debug, Default, Clone, Copy)]
pub struct KeywordModel;

impl ModelProvider for KeywordModel {
    fn name(&self) -> &str {
        "keyword-model"
    }

    async fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        let Some(last) = request.messages.last() else {
            return Ok(ModelResponse::text("Ask me something."));
        };
        let response = match last.role {
            Role::Tool => answer_from_tool(last),
            _ => {
                let question = last.content.to_lowercase();
                if question.contains("ticket") {
                    let title: String = last.content.chars().take(80).collect();
                    ModelResponse::tool_call("call-1", "create_ticket", json!({ "title": title }))
                } else if question.contains("status") {
                    ModelResponse::tool_call(
                        "call-1",
                        "read_project_status",
                        json!({ "project": "liga" }),
                    )
                } else {
                    ModelResponse::text("I can report the project status or open a ticket.")
                }
            }
        };
        let prompt_chars: usize = request.messages.iter().map(|m| m.content.len()).sum();
        Ok(response.with_usage((prompt_chars / 4) as u64, 16))
    }
}

fn answer_from_tool(result: &Message) -> ModelResponse {
    let value: serde_json::Value = serde_json::from_str(&result.content).unwrap_or_default();
    if let Some(code) = value.get("error").and_then(|c| c.as_str()) {
        return ModelResponse::text(format!("I could not do that ({code})."));
    }
    if let Some(build) = value.get("build").and_then(|b| b.as_str()) {
        return ModelResponse::text(format!(
            "Project {} is at milestone {} with {} open issues; the build is {build}.",
            value["project"].as_str().unwrap_or("?"),
            value["milestone"].as_str().unwrap_or("?"),
            value["open_issues"],
        ));
    }
    ModelResponse::text(format!("Done: ticket #{} was created.", value["id"]))
}

// ------------------------------------------------------------------ tools

/// Arguments of `read_project_status`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusArgs {
    /// Project name.
    pub project: String,
}

/// Result of `read_project_status`.
#[derive(Debug, Serialize)]
pub struct ProjectStatus {
    project: String,
    milestone: &'static str,
    open_issues: u32,
    build: &'static str,
}

/// Read-only tool.
#[derive(Debug, Clone, Copy)]
pub struct ReadProjectStatus;

impl Tool for ReadProjectStatus {
    type Args = StatusArgs;
    type Output = ProjectStatus;

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "read_project_status",
            "Returns milestone, open issues and build state of a project",
        )
        .parameters(json!({
            "type": "object",
            "properties": { "project": { "type": "string" } },
            "required": ["project"],
            "additionalProperties": false
        }))
        .permission("project.read")
    }

    async fn call(&self, args: StatusArgs, _ctx: &ToolContext) -> Result<ProjectStatus, ToolError> {
        if args.project != "liga" {
            return Err(ToolError::new(format!(
                "unknown project `{}`",
                args.project
            )));
        }
        Ok(ProjectStatus {
            project: args.project,
            milestone: "0.1 — web MVP",
            open_issues: 7,
            build: "passing",
        })
    }
}

/// A created ticket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ticket {
    /// Identifier.
    pub id: u64,
    /// Title.
    pub title: String,
}

/// Ticket store (the side effect `create_ticket` performs).
#[derive(Debug, Default)]
pub struct Tickets {
    tickets: Mutex<Vec<Ticket>>,
}

impl Tickets {
    /// Every ticket.
    pub fn list(&self) -> Vec<Ticket> {
        self.tickets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn create(&self, title: String) -> Ticket {
        let mut tickets = self.tickets.lock().unwrap_or_else(PoisonError::into_inner);
        let ticket = Ticket {
            id: tickets.len() as u64 + 1,
            title,
        };
        tickets.push(ticket.clone());
        ticket
    }
}

/// Arguments of `create_ticket`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TicketArgs {
    /// Ticket title.
    pub title: String,
}

/// Tool with a side effect; requires `tickets.write`.
#[derive(Debug, Clone)]
pub struct CreateTicket {
    tickets: Arc<Tickets>,
}

impl Tool for CreateTicket {
    type Args = TicketArgs;
    type Output = Ticket;

    fn spec(&self) -> ToolSpec {
        ToolSpec::new("create_ticket", "Opens a ticket in the issue tracker")
            .parameters(json!({
                "type": "object",
                "properties": { "title": { "type": "string", "maxLength": 80 } },
                "required": ["title"],
                "additionalProperties": false
            }))
            .permission("tickets.write")
    }

    async fn call(&self, args: TicketArgs, _ctx: &ToolContext) -> Result<Ticket, ToolError> {
        let title = args.title.trim();
        if title.is_empty() || title.chars().count() > 80 {
            return Err(ToolError::new("title must have 1-80 characters"));
        }
        Ok(self.tickets.create(title.to_owned()))
    }
}

// ----------------------------------------------------------------- module

/// The `[assistant]` configuration section.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AssistantConfig {
    /// Grants `tickets.write` to the agent.
    pub allow_ticket_creation: bool,
    /// Model calls per question.
    pub max_steps: u32,
    /// Time per question.
    pub timeout_ms: u64,
}

impl Default for AssistantConfig {
    fn default() -> Self {
        Self {
            allow_ticket_creation: false,
            max_steps: 4,
            timeout_ms: 10_000,
        }
    }
}

impl Validate for AssistantConfig {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors
            .check(
                (1..=16).contains(&self.max_steps),
                "max_steps",
                "range",
                "must be 1-16",
            )
            .check(
                (1..=120_000).contains(&self.timeout_ms),
                "timeout_ms",
                "range",
                "must be 1-120000",
            );
        errors.into_result()
    }
}

/// Builds the agent and exposes it over HTTP.
#[derive(Debug, Clone, Copy)]
pub struct AssistantModule;

impl AetherModule for AssistantModule {
    fn name(&self) -> &'static str {
        "assistant"
    }

    fn register(&self, ctx: &mut AetherModuleContext<'_>) -> Result<(), AetherModuleError> {
        let config: AssistantConfig = ctx.config_section("assistant")?;
        let tickets = Arc::new(Tickets::default());

        let mut policy = Policy::deny_by_default().grant("project.read");
        if config.allow_ticket_creation {
            policy = policy.grant("tickets.write");
        }
        let agent = AetherAgent::builder()
            .provider(KeywordModel)
            .system_prompt("You help the team track the project. Use tools; never invent data.")
            .tool(ReadProjectStatus)
            .tool(CreateTicket {
                tickets: tickets.clone(),
            })
            .policy(policy)
            .max_steps(config.max_steps)
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| AetherModuleError::with_source("invalid agent configuration", e))?;

        ctx.provide(agent)?;
        ctx.provide_arc(tickets)?;
        ctx.contribute(Routes::new().route("/assistant/ask", post(ask)));
        Ok(())
    }
}

/// Body of `POST /assistant/ask`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    /// The question, 1–2000 characters.
    pub question: String,
}

impl Validate for Question {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let len = self.question.trim().chars().count();
        let mut errors = ValidationErrors::new();
        errors.check(
            (1..=2_000).contains(&len),
            "question",
            "length",
            "must have 1-2000 characters",
        );
        errors.into_result()
    }
}

/// One tool call in the response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolUse {
    /// Tool name.
    pub tool: String,
    /// `succeeded`, `denied`, `unknown_tool`, ...
    pub outcome: String,
}

/// Response of `POST /assistant/ask`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    /// Final answer.
    pub answer: String,
    /// Model calls used.
    pub steps: u32,
    /// Tools the agent tried to use.
    pub tool_calls: Vec<ToolUse>,
}

fn outcome_label(outcome: &ToolCallOutcome) -> &'static str {
    match outcome {
        ToolCallOutcome::Succeeded { .. } => "succeeded",
        ToolCallOutcome::Denied { .. } => "denied",
        ToolCallOutcome::UnknownTool => "unknown_tool",
        ToolCallOutcome::InvalidArguments => "invalid_arguments",
        ToolCallOutcome::InputTooLarge => "input_too_large",
        ToolCallOutcome::Failed => "failed",
        ToolCallOutcome::TimedOut => "timed_out",
        _ => "other",
    }
}

async fn ask(agent: Dep<AetherAgent>, ValidJson(body): ValidJson<Question>) -> ApiResult<Json<Answer>> {
    let outcome = agent.run(body.question).await.map_err(|err| match err {
        AetherAgentError::Timeout { .. } => ApiError::timeout(),
        AetherAgentError::Provider(source) => {
            ApiError::service_unavailable("the model provider is unavailable").with_source(source)
        }
        other => ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "agent_gave_up",
            other.to_string(),
        ),
    })?;
    Ok(Json(Answer {
        answer: outcome.answer,
        steps: outcome.steps,
        tool_calls: outcome
            .tool_calls
            .iter()
            .map(|record| ToolUse {
                tool: record.tool.clone(),
                outcome: outcome_label(&record.outcome).to_owned(),
            })
            .collect(),
    }))
}

/// The application.
pub fn app_builder() -> AetherApplicationBuilder {
    AetherApplication::builder()
        .name("ai-tool-server")
        .optional_config_file("ai-tool-server.toml")
        .env_prefix("ASSISTANT")
        .module(AssistantModule)
}
