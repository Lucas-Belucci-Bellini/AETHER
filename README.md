# AETHER — framework modular para aplicações Rust

AETHER combina HTTP, módulos, tarefas em segundo plano e agentes de IA com permissões explícitas num framework modular escrito em Rust.

Os nomes oficiais da API pública seguem o repositório: `AetherApplication`, `AetherApplicationBuilder`, `AetherModule`, `AetherAppContext`, `AetherConfig`, `AetherTaskRunner`, `AetherApiError` e `AetherAgent`. O mesmo padrão é usado nos tipos de configuração, validação, eventos, tarefas, HTTP e ferramentas de IA. Os nomes genéricos continuam disponíveis para compatibilidade; os nomes dos crates `liga-*` permanecem inalterados nesta etapa.

```rust
use liga::prelude::*;

struct Hello;

impl AetherModule for Hello {
    fn name(&self) -> &'static str { "hello" }

    fn register(&self, ctx: &mut AetherModuleContext<'_>) -> Result<(), AetherModuleError> {
        ctx.contribute(Routes::new().route("/hello", get(|| async { "olá, AETHER" })));
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), liga::AetherError> {
    liga::init_tracing();
    AetherApplication::builder()
        .name("hello")
        .env_prefix("HELLO")          // HELLO__SERVER__ADDR=0.0.0.0:8080
        .module(Hello)
        .build()                      // valida config, ordena e inicia módulos
        .await?
        .serve()                      // Ctrl+C → drena requisições → shutdown reverso
        .await
}
```

## Estado

**v0.1.0 (pré-release, não publicado).** As fases 0 a 4 do [roadmap](docs/04-roadmap.md) estão implementadas e testadas. O que falta está em [Próximos passos](#próximos-passos).

| Verificação | Resultado |
|---|---|
| `cargo test --workspace --all-features` | 80 testes passando |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | sem avisos |
| `cargo fmt --all -- --check` | ok |
| `cargo check -p liga --no-default-features` (e cada feature isolada) | ok |
| MSRV 1.94 (`cargo +1.94.1 check --workspace --all-features`) | ok |

## Os três pilares

| Pilar | Crate | O que entrega |
|---|---|---|
| Núcleo | `liga-core` | Config em camadas (defaults < arquivo < ambiente) validada no startup, `AppContext`, estado tipado, `Shutdown`, `Secret<T>` |
| Módulos | `liga-modules` | Trait `AetherModule`, ordenação topológica, detecção de duplicatas, dependências ausentes e ciclos, start com rollback, shutdown reverso, barramento de eventos em memória |
| Web | `liga-web` | Axum + Tokio, envelope JSON para erros, `ApiEnvelope<T>`/`ApiMeta` para sucesso versionado v1, extractors consistentes, request-id, tracing, timeout, limites e `TestClient` sem porta |
| Automação | `liga-tasks` | Runner local com concorrência limitada, timeout por tentativa, retry só para erros transitórios (backoff exponencial + jitter), cancelamento e shutdown com prazo |
| IA | `liga-ai` | `ModelProvider` neutro, `Tool` tipada com permissões, `Policy` deny-by-default, `AetherAgent` limitado por passos, tempo, chamadas e tamanho, `ScriptedProvider` para testes |
| Facade | `liga` | `AetherApplication::builder()`, `prelude`, features `web` (padrão), `logging` (padrão), `tasks`, `ai` e `full` |

O `liga-core` não depende de HTTP nem de IA. Com `--no-default-features`, o `liga` não compila axum nem hyper.

## Exemplos

```bash
cargo run -p hello-api
```

| Exemplo | Mostra |
|---|---|
| [`hello-api`](examples/hello-api) | O primeiro marco: catálogo com config validada, `ValidJson`, 404/409/422 padronizados |
| [`modular-api`](examples/modular-api) | Três módulos (`accounts` → `audit`, `notifications`) ligados por eventos tipados; e-mail em background com retry; shutdown drena a fila |
| [`ai-tool-server`](examples/ai-tool-server) | `POST /assistant/ask` com agente: a tool de leitura é liberada, a de escrita é negada até o operador liberar via config, mesmo sob tentativa de prompt injection |

Os exemplos usam um modelo determinístico local: nenhum precisa de rede ou chave de API.

## Erros HTTP

Toda falha, inclusive 404, 405, JSON malformado, timeout e erro interno, usa o mesmo formato:

```json
{ "error": { "code": "validation_failed", "message": "request validation failed",
             "details": { "fields": [{ "field": "name", "code": "required", "message": "..." }] },
             "request_id": "65c92edc-8349-4ac7-aeb6-867cee64fac6" } }
```

Erros internos retornam só `internal_error`. A causa vai para o log com o mesmo `request_id` e nunca para o cliente.

## Configuração

```toml
# hello-api.toml (opcional)
[server]
addr = "127.0.0.1:3000"
request_timeout_ms = 10000
body_limit_bytes = 1048576
max_concurrent_requests = 512
shutdown_timeout_ms = 10000

[catalog]
max_products = 1000
```

As variáveis de ambiente vencem o arquivo: `HELLO__CATALOG__MAX_PRODUCTS=50`. Campos desconhecidos, tipos errados e valores fora da faixa derrubam o `build()` com o caminho exato (`catalog.max_products`), antes de qualquer módulo iniciar.

## Documentação do projeto

1. [Visão, escopo e não objetivos](docs/01-visao-e-escopo.md)
2. [Arquitetura](docs/02-arquitetura.md)
3. [Workspace e crates](docs/03-workspace-e-crates.md)
4. [Roadmap por fases](docs/04-roadmap.md)
5. [API pública](docs/05-api-publica-proposta.md)
6. [Testes e qualidade](docs/06-plano-de-testes.md)
7. [Segurança e observabilidade](docs/07-seguranca-observabilidade.md)
8. [Decisões técnicas (ADRs)](docs/08-decisoes-tecnicas-adr.md)
9. [Critérios de aceite](docs/09-criterios-de-aceite.md)
10. [Referências](docs/10-fontes-e-referencias.md)
11. [Backlog de issues](templates/issues/README.md)

## Próximos passos

- Adaptador real de modelo (`liga-ai-openai` ou adaptador HTTP compatível), com teste real opt-in.
- `liga-cli` (scaffolding de projeto e módulo).
- OpenAPI gerado a partir das rotas.
- Benchmarks reproduzíveis contra Axum puro (fase 5).
- CI no GitHub: o workflow `Rust CI` já está ativo em `.github/workflows/ci.yml` e executa formatação, `cargo check`, testes com todas as features, Clippy com `-D warnings` e validação da MSRV 1.94. Para repetir a matriz ampliada localmente (incluindo isolamento de dependências), use `bash scripts/verify.sh`.
- Revisar a licença escolhida (`MIT OR Apache-2.0`).

## Licença

`MIT OR Apache-2.0`, a convenção do ecossistema Rust.
