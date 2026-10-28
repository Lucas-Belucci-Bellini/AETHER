//! `cargo run -p modular-api`, then:
//!
//! ```text
//! curl -X POST localhost:3000/accounts -H 'content-type: application/json' -d '{"email":"ana@example.com"}'
//! curl localhost:3000/audit
//! curl localhost:3000/notifications
//! ```

#[tokio::main]
async fn main() -> Result<(), liga::AetherError> {
    liga::init_tracing();
    modular_api::app_builder().build().await?.serve().await
}
