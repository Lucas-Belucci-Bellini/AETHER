# 8. Decisões técnicas (ADRs)

Um ADR é uma decisão técnica curta, datada e revisável. As decisões abaixo foram aceitas em 2026-10-25, junto com a implementação v0.1.0; cada uma aponta onde vive no código.

## ADR-0001: um workspace com crates por responsabilidade

**Status:** Aceito

**Decisão:** seis crates (`liga-core`, `liga-modules`, `liga-web`, `liga-tasks`, `liga-ai` e o facade `liga`) mais três exemplos. O crate de "automação" virou `liga-tasks`. Os adaptadores de IA e a CLI ficam para depois.

**Motivo:** dependências opcionais, fronteiras de arquitetura verificáveis e testes isolados.

**Consequência:** `cargo tree -p liga --no-default-features` não contém axum nem hyper. A regra "core não importa web nem IA" é garantida pelo grafo de dependências, não por convenção.

## ADR-0002: Axum/Tokio como base HTTP

**Status:** Aceito

**Decisão:** `liga-web` usa Axum 0.8 + Tokio. O middleware (request-id, tracing, timeout e concorrência) é escrito com `axum::middleware::from_fn`, em vez de `tower-http`.

**Motivo:** os middlewares próprios devolvem o mesmo envelope JSON de erro (um timeout responde `503 {"error":{"code":"timeout"}}`, não um corpo vazio) e evitam uma dependência a mais.

**Consequência:** os tipos do Axum são reexportados (`liga::web::axum`) e fazem parte da API pública. Um upgrade incompatível do Axum é um release incompatível do `liga-web`.

## ADR-0003: núcleo neutro de provedor de IA

**Status:** Aceito

**Decisão:** o trait `ModelProvider` tem um único método `complete`. O `ScriptedProvider` (em `liga_ai::testing`) torna os testes determinísticos. Os adaptadores reais ficam em crates separados.

**Consequência:** nenhum teste do CI precisa de rede ou chave. O primeiro adaptador real ainda está pendente.

## ADR-0004: plugins nativos dinâmicos fora do MVP

**Status:** Aceito

**Decisão:** extensões são crates Rust registradas explicitamente com `.module(...)`. Nada é carregado em runtime.

## ADR-0005: eventos em memória com semântica definida

**Status:** Aceito

**Decisão (`liga_modules::events`):**

| Pergunta | Resposta |
|---|---|
| Síncrono ou assíncrono? | `publish` é `async` e espera todos os handlers |
| Ordem | Sequencial, na ordem de inscrição |
| Falha de um handler | Isolada: os outros rodam; todas as falhas voltam em `PublishError` e vão para o log |
| Panic ou handler travado | Viram falha (cada handler roda numa task própria, com timeout padrão de 30 s) |
| Reentrância | Permitida (nenhum lock fica preso durante os handlers) |
| Durabilidade | Nenhuma: só em memória |

## ADR-0006: recursos avançados via features

**Status:** Aceito

**Decisão:** o facade `liga` tem `default = ["web", "logging"]`, e `tasks`, `ai` e `full` são opt-in. O sistema de módulos está sempre presente.

**Consequência:** o `scripts/verify.sh` compila cada feature isolada e a combinação `full`.

## ADR-0007: segurança de execução para tools

**Status:** Aceito

**Decisão (`liga_ai::Agent`):**

- Toda tool declara pelo menos uma `Permission`. Uma tool sem permissão é rejeitada no `build()`, o que fecha o atalho que contornaria o deny-by-default.
- A `Policy` começa vazia, e as concessões vêm só do código da aplicação.
- Antes de executar, o agente verifica: tool registrada? Permissões concedidas? Argumentos dentro do limite de tamanho? Argumentos desserializam no tipo `Args`?
- Uma chamada recusada não executa nada. O modelo recebe um erro JSON e pode se recuperar.
- Cada execução é limitada por `max_steps`, `timeout`, `tool_timeout`, `max_tool_calls` e pelos tamanhos de entrada e saída.
- Os logs registram nomes, IDs, resultados e tamanhos, nunca o conteúdo de prompts ou tools.

**Verificação:** os testes `prompt_injection_cannot_run_ungranted_tools` (em `liga-ai`) e `side_effects_are_denied_unless_configured` (em `ai-tool-server`).

## ADR-0008: nome "Liga"

**Status:** Aceito

**Decisão:** o nome do projeto é **Liga**, e os crates seguem o padrão `liga-*`.

**Motivo:** em português, *liga* é a mistura de metais que resulta num material mais forte, e o projeto mistura Web, módulos e IA sobre a base de Rust (ferrugem/ferro). O nome é curto e fácil de digitar em `use liga::prelude::*`. Em 2026-10-24, `liga` e `liga-core` estavam livres no crates.io, enquanto `ferrum` e `forja` estavam ocupados.

**Consequência:** reservar os nomes no crates.io antes do primeiro release público.

## ADR-0009: edição 2024, MSRV 1.94

**Status:** Aceito

**Decisão:** edição Rust 2024 e `rust-version = "1.94"`, com o resolver 3, que leva o MSRV em conta ao escolher versões de dependências.

**Motivo:** os traits async nativos (`fn ... -> impl Future + Send`, escritos como `async fn` nas implementações) substituem o `async_trait`. A versão 1.94 é a mais antiga instalada e verificada no ambiente de desenvolvimento.

**Consequência:** o `Module`, o `ModelProvider` e o `Tool` não exigem macros. O armazenamento heterogêneo usa traits internas `Dyn*` com futures em `Box`.

## ADR-0010: configuração em camadas e validada no startup

**Status:** Aceito

**Decisão (`liga_core::config`):** a precedência é **defaults < arquivos (na ordem adicionada) < variáveis de ambiente**. As variáveis usam `PREFIXO__SECAO__CHAVE`. Os valores de ambiente continuam strings e só viram número ou bool quando o campo de destino pede, de modo que a senha `12345` continua `String`. Os erros trazem o caminho completo (`server.port`). `Validate` roda depois da desserialização. O `Debug` da config mostra só as seções, e um erro de parse não ecoa o conteúdo do arquivo.

**Consequência:** `#[serde(deny_unknown_fields)]` nas seções transforma um erro de digitação (`MAX_PRODCTS`) em falha no startup, em vez de valor ignorado.

## ADR-0011: `register` síncrono, `start`/`shutdown` assíncronos

**Status:** Aceito

**Decisão:** `register` não faz I/O: lê config, fornece estado tipado (`provide`), contribui valores às integrações (`contribute`, como as rotas) e se inscreve em eventos. `start` adquire recursos e `shutdown` os libera em ordem reversa. Se um `start` falhar, os módulos já iniciados recebem `shutdown`.

**Motivo:** toda a validação de wiring (dependências, estado duplicado, rotas em conflito) acontece antes de abrir qualquer recurso.

## ADR-0012: envelope único de erro HTTP

**Status:** Aceito

**Decisão:** `{"error":{"code","message","details"?,"request_id"?}}` para toda resposta de erro, inclusive as geradas pelo framework (404, 405, 413, 415, 400/422 de corpo e timeout). Qualquer `std::error::Error` vira `500 internal_error` via `?`, e a causa só vai para o log.

## Pendentes antes do primeiro release público

- Confirmar a licença (`MIT OR Apache-2.0` está declarada nos `Cargo.toml`, mas os arquivos de licença ainda não foram adicionados).
- Política de semver e estabilização de APIs.
- Autenticação/autorização: pontos de extensão ou componentes oficiais.
- OpenAPI no 0.2 ou depois.
