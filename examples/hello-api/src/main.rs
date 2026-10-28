//! `cargo run -p hello-api`, then `curl http://127.0.0.1:3000/products`.

#[tokio::main]
async fn main() -> Result<(), liga::AetherError> {
    liga::init_tracing();
    hello_api::app_builder().build().await?.serve().await
}
