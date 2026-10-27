# 3. Workspace e divisão de crates

O workspace implementado (v0.1.0):

```text
liga/
├── Cargo.toml                 # workspace, dependências e lints compartilhados
├── rust-toolchain.toml        # stable + rustfmt + clippy
├── clippy.toml                # unwrap/expect permitidos só em testes
├── crates/
│   ├── liga-core/             # config, AppContext, estado tipado, Shutdown, Secret, Validate
│   ├── liga-modules/          # Module, ModuleSet, lifecycle, EventBus
│   ├── liga-web/              # Axum: Routes, ApiError, extractors, middleware, TestClient
│   ├── liga-tasks/            # TaskRunner: concorrência, timeout, retry, cancelamento
│   ├── liga-ai/               # ModelProvider, Tool, Policy, Agent, ScriptedProvider
│   └── liga/                  # facade: Application, prelude, features; tests/ de ponta a ponta
├── examples/
│   ├── hello-api/             # primeiro marco: catálogo
│   ├── modular-api/           # módulos + eventos + tarefas
│   └── ai-tool-server/        # agente com tools permissionadas via HTTP
├── docs/                      # este plano
└── scripts/verify.sh          # portão de qualidade local: fmt, clippy, testes, features, MSRV
```

Planejados e ainda não criados: `liga-ai-openai` (adaptador real) e `liga-cli` (scaffolding).

## Dependências candidatas

| Área | Candidatos | Política |
|---|---|---|
| Runtime assíncrono | Tokio | Usar recursos por feature; evitar `full` automaticamente na biblioteca. |
| HTTP e rotas | Axum + Hyper | Manter a API pública desacoplada de detalhes internos quando viável. |
| Middleware | Tower + tower-http | Preferir composição de serviços em vez de criar um sistema paralelo. |
| Serialização | Serde | Limitar macros e formatos habilitados ao necessário. |
| Erros | thiserror; anyhow em aplicações/exemplos | Erros públicos tipados; evitar apagar tipos nos contratos de biblioteca. |
| Logs e tracing | tracing + tracing-subscriber | Contexto estruturado e redaction de campos sensíveis. |
| CLI | clap | Opcional; não obrigatório para usar o framework. |
| Integração de modelo | async-openai ou adaptador HTTP independente | Feature separada; nenhuma chave de API em código. |
| Testes | tokio-test, tower::ServiceExt, testes nativos do Rust | Exemplos críticos também rodam em CI. |

As versões não devem ser copiadas cegamente deste documento. No início da implementação, verificar versões estáveis, MSRV, licenças, advisories e compatibilidade direta entre crates; registrar o resultado em `Cargo.lock` para aplicações e na política de dependências do workspace.

## Política de features

Exemplo conceitual:

```toml
[features]
default = ["web"]
web = ["dep:axum", "dep:tower", "dep:tokio"]
modules = []
automation = ["dep:tokio"]
ai = ["automation"]
ai-openai = ["ai", "dep:async-openai"]
cli = ["dep:clap"]
full = ["web", "modules", "automation", "ai", "ai-openai", "cli"]
```

Este trecho é um rascunho: os nomes dos recursos `dep:` e a propagação de features devem ser validados no `Cargo.toml` real de cada crate. Evite dependências opcionais redundantes e recursos que ativem bibliotecas que o usuário não solicitou.

## MSRV e edição

- Selecionar uma versão mínima do Rust (MSRV) conscientemente.
- Usar uma edição Rust atual e suportada após verificar o toolchain disponível no CI.
- Testar a MSRV e a stable atual no pipeline; adotar beta/nightly apenas em tarefas específicas, não como requisito padrão.
- Documentar política de breaking changes e versionamento semântico.

## Convenções de organização

- Cada crate tem README curto explicando o seu papel e exemplo mínimo.
- `core` deve permanecer pequeno e estável.
- Evitar ciclos de dependências entre crates.
- `pub` só quando a API for intencionalmente pública.
- Testar feature flags isoladas e combinações comuns.
