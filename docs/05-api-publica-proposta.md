# 5. API pública (v0.1.0)

Os trechos abaixo compilam: todos foram tirados do código e dos exemplos, que `scripts/verify.sh` compila e testa. A documentação completa de cada item está nos comentários `///` (`cargo doc --workspace --all-features --open`).

## Aplicação

```rust
use liga::prelude::*;

#[tokio::main]
async fn main() -> Result<(), liga::Error> {
    liga::init_tracing();
    Application::builder()
        .name("catalog-api")
        .optional_config_file("catalog.toml")
        .env_prefix("CATALOG")               // CATALOG__SERVER__ADDR=0.0.0.0:8080
        .module(CatalogModule)
        .build()                             // config → resolve → register → start → router
        .await?
        .serve()                             // bind, sinais, drenagem, shutdown reverso
        .await
}
```

`build()` falha (sem servir nada) quando há config inválida, módulo duplicado, dependência ausente, ciclo, estado tipado duplicado, falha de `start` ou rotas conflitantes. Nos dois últimos casos, os módulos já iniciados recebem `shutdown` antes de o erro voltar.

Em testes, `app.test_client()` exercita o router inteiro sem abrir porta, e `app.shutdown().await` encerra os módulos.

## Módulos

```rust
pub trait Module: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn dependencies(&self) -> &'static [&'static str] { &[] }
    fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> { Ok(()) }
    fn start(&self, ctx: &AppContext) -> impl Future<Output = Result<(), ModuleError>> + Send;
    fn shutdown(&self, ctx: &AppContext) -> impl Future<Output = Result<(), ModuleError>> + Send;
}
```

Nas implementações, `start` e `shutdown` são escritos como `async fn`. Durante o `register`:

```rust
let config: CatalogConfig = ctx.config_section("catalog")?; // desserializa + Validate
ctx.provide(Catalog::new(config.max_products))?;            // estado tipado global
ctx.contribute(Routes::new().route("/products", get(list))); // rotas para o liga-web
ctx.events().subscribe(|e: Arc<AccountCreated>| async move { Ok(()) });
```

## Handlers

```rust
async fn show(catalog: Dep<Catalog>, Path(id): Path<u64>) -> ApiResult<Json<Product>> {
    catalog.get(id).map(Json).ok_or_else(|| ApiError::not_found(format!("product {id} not found")))
}

async fn create(catalog: Dep<Catalog>, ValidJson(new): ValidJson<NewProduct>)
    -> ApiResult<(StatusCode, Json<Product>)> { /* ... */ }
```

`Dep<T>` lê o estado tipado. `Json`, `ValidJson`, `Path` e `Query` rejeitam com o envelope padrão de erro.

## Eventos

```rust
let delivery = events.publish(AccountCreated { id }).await?; // Err(PublishError) se algum handler falhar
```

A semântica está no ADR-0005: em memória, sequencial, falhas isoladas e reportadas.

## Tarefas

```rust
let handle = runner.spawn(
    TaskSpec::new("welcome-email")
        .timeout(Duration::from_secs(10))
        .retry(RetryPolicy::exponential(3, Duration::from_millis(50))),
    |task: TaskContext| async move {
        send(task.attempt()).await.map_err(|e| TaskError::transient("gateway down").with_source(e))
    },
)?;
handle.status();          // Queued | Running | Retrying | Succeeded | Failed | TimedOut | Cancelled | Panicked
handle.cancel();
runner.shutdown(Duration::from_secs(5)).await; // drena; cancela o que passar do prazo
```

## IA e ferramentas

```rust
let agent = Agent::builder()
    .provider(model)                                  // qualquer ModelProvider
    .tool(ReadProjectStatus)                          // declara Permission "project.read"
    .tool(CreateTicket { tickets })                   // declara Permission "tickets.write"
    .policy(Policy::deny_by_default().grant("project.read"))
    .max_steps(4)
    .timeout(Duration::from_secs(10))
    .build()?;

let outcome = agent.run("Qual o status do projeto?").await?;
outcome.answer;      // resposta final
outcome.tool_calls;  // auditoria: Succeeded | Denied | UnknownTool | InvalidArguments | ...
```

## Estilo de API (mantido)

- Builders para opções opcionais; nenhuma macro obrigatória para rotas, módulos ou tools.
- Nenhum `unwrap()`/`expect()` em código de biblioteca (o clippy acusa como aviso, e `scripts/verify.sh` roda com `-D warnings`).
- Enums de erro e de estado públicos são `#[non_exhaustive]`.
- Os tipos do Axum expostos por `liga-web` estão documentados como parte da API (ADR-0002).
