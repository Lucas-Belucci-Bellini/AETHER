//! `cargo run -p ai-tool-server`, then:
//!
//! ```text
//! curl -X POST localhost:3000/assistant/ask -H 'content-type: application/json' \
//!      -d '{"question":"What is the project status?"}'
//! ```

#[tokio::main]
async fn main() -> Result<(), liga::Error> {
    liga::init_tracing();
    ai_tool_server::app_builder().build().await?.serve().await
}
