# Roadmap técnico — AETHER (auditoria coordenada, outubro de 2026)

> Estado desta revisão: inspeção estática de manifests, README e arquivos centrais de implementação via GitHub. **Nenhum comando de teste/build foi executado nesta auditoria.** Não tratar os resultados listados no README como resultados recém-verificados.
>
> Branch de trabalho: `roadmap/coordinated-audit-2026-10`. A branch `main` não foi alterada.

## 1. Papel do AETHER no ecossistema

AETHER deve ser a camada de composição de aplicações e automação: núcleo tipado, módulos, HTTP opcional, tarefas locais e agentes de IA com permissões explícitas. Não deve absorver o papel de frontend WebAssembly do ATLAS nem duplicar a base HTTP mínima do TITAN. A implementação atual usa nomes internos Liga/`liga-*`; preservar esses nomes até uma decisão explícita de identidade.

## 2. Estado de código observado

- Workspace virtual Cargo com `resolver = "3"`, edition 2024, `rust-version = "1.94"`, crates sob `crates/*` e exemplos sob `examples/*`.
- `liga-core`: módulos de configuração, contexto/estado tipado, segredos, shutdown e validação; o crate declara não conhecer HTTP nem IA.
- `liga-modules`: resolução de dependências entre módulos, registro, startup, rollback após falha e shutdown reverso; eventos em memória.
- `liga-web`: Axum/Tokio, rotas contribuídas por módulos, erros JSON, extractors, request ID/tracing, timeout, limites de concorrência/body e TestClient.
- `liga-tasks`: runner em processo com concorrência limitada, timeout, retry de falhas transitórias, cancelamento e estados observáveis. A própria documentação do crate diz que não é durável.
- `liga-ai`: contrato `ModelProvider`, tools tipadas, policy deny-by-default e limites por execução. O provider de teste é determinístico; adaptador real não foi encontrado nas crates inspecionadas.
- `liga`: facade com features opcionais `web`, `logging`, `tasks`, `ai` e `full`.
- `Cargo.lock` existe. Os caminhos `.github/workflows/ci.yml` e `.github/workflows/rust.yml` não foram encontrados; não se conclui daí que não haja workflow com outro nome.
- O README aponta para documentação detalhada em `docs/01...` a `docs/10...` e backlog em `templates/issues/README.md`. A existência/conteúdo de todos esses arquivos ainda precisa ser reconciliada com os caminhos reais; o caminho `docs/01-architecture.md` consultado não existe, enquanto o README lista `docs/01-visao-e-escopo.md`.

## 3. Lacunas e riscos para validar

1. Executar a matriz real de build/teste; comparar com a declaração do README de 80 testes e clippy limpo.
2. Conferir features isoladas e `--no-default-features`, incluindo exemplos que podem ativar features implicitamente.
3. Inspecionar semântica de shutdown: drenagem de requisições, timeout de shutdown, cancelamento cooperativo e rollback de módulos.
4. Testar limites/retries do runner, especialmente panic, cancelamento durante backoff, timeout e shutdown com tarefas ativas.
5. Auditar policy de IA contra prompt injection, chamadas duplicadas, tools desconhecidas, argumentos grandes, saída truncada, timeout e exposição de erros.
6. Definir e testar contrato de adaptação de provider real; não acoplar API de fornecedor ao núcleo.
7. Revisar a superfície pública e política de compatibilidade: Axum está reexportado, portanto sua atualização pode ser breaking para consumidores.
8. Verificar documentação referenciada, licença MIT OR Apache-2.0, metadados, exemplos e ausência de segredos no histórico.
9. Confirmar CI, política de dependências, advisories e reprodução do build com lockfile.

## 4. Roadmap por fases

### Fase A — Baseline verificável (P0)
- Rodar e registrar: `cargo fmt --all -- --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo check --workspace --locked`.
- Rodar matriz de features: sem defaults; `web`, `logging`, `tasks`, `ai` isoladas; `full`.
- Confirmar MSRV real com toolchain 1.94 e documentar se a versão exata é necessária.
- Conferir caminhos e links da documentação; corrigir apenas após verificar destinos existentes.
- Criar CI de pull request/push na branch própria, sem workflows com permissões de escrita desnecessárias.

**Aceite:** comandos executados com logs, toolchain registrada, falhas classificadas como código/ambiente/dependência e baseline reproduzível.

### Fase B — Contratos e robustez do runtime (P1)
- Adicionar testes de integração para ordem de módulos, ciclo, dependência ausente, rollback parcial e shutdown reverso.
- Testar concorrência e limites de tarefa sob cancelamento/panic/timeouts; definir claramente o que ocorre com tarefas já iniciadas.
- Testar serialização e redaction de `Secret<T>`, configuração inválida e mensagens de erro sem dados sensíveis.
- Documentar garantias e não garantias do EventBus em memória.
- Verificar que erros internos HTTP não vazam detalhes; garantir request ID consistente entre resposta e logs.
- Cobrir feature combinations e API pública com exemplos compiláveis.

**Aceite:** contratos de ciclo de vida e erros documentados e protegidos por testes determinísticos.

### Fase C — Provider real e segurança de IA (P1)
- Definir trait/adapter contract e configuração segura para provider externo, com timeouts e limites de payload.
- Adicionar integração real apenas opt-in por variável de ambiente/secret de CI; suite padrão continua sem rede/chaves.
- Testar redaction de prompts, tool arguments/results, permission deny-by-default e erros do provider.
- Definir política para retries do provider, rate limit, custos, token usage e idempotência de tools.
- Escrever threat model curto e exemplos de tools read-only vs mutating com autorização explícita.

**Aceite:** provider real opcional; testes padrão determinísticos; nenhum segredo nos logs; permissões sempre decididas pelo host.

### Fase D — Usabilidade de framework (P2)
- Projetar `liga-cli` somente depois de estabilizar nomes/estrutura: scaffold, geração de módulo e validação de configuração.
- Gerar OpenAPI a partir de rotas apenas se a integração não exigir duplicar manualmente os contratos.
- Criar benchmarks reprodutíveis de overhead contra Axum puro e medir memória/latência antes de otimizar.
- Definir compatibilidade semver e critérios para primeira versão 1.0.

**Aceite:** decisões em ADR, benchmarks com instruções reproduzíveis, nenhuma promessa de performance sem medições.

## 5. Ordem recomendada de execução

1. Baseline real de testes e CI.
2. Auditoria dos contratos de shutdown, tarefas e módulos.
3. Hardening da IA e contrato de provider.
4. Correções de documentação e exemplos.
5. CLI/OpenAPI/benchmarks após ADRs.

## 6. Dependências com ATLAS e TITAN

- Não depender de ATLAS; a interface web pode consumir uma API servida por AETHER, mas isso deve ser exemplo de integração, não dependência de build.
- Para integração HTTP com TITAN, definir apenas contratos de transporte/erro compartilhados se existir benefício concreto; não criar dependência circular nem fundir os frameworks.
- Qualquer crate comum futura exige ADR, dono claro e justificativa contra duplicação de abstrações.

## 7. Fora de escopo por enquanto

- Persistência de tarefas, distribuição multi-processo e garantias exactly-once.
- Provider real ligado por padrão ou chamadas de rede na suíte padrão.
- CLI, OpenAPI e benchmarks antes do baseline.
- Mudanças de nome dos crates/repositório sem decisão de identidade.

## 8. Critério de conclusão da auditoria

O roadmap só vira baseline oficial após: (a) comandos executados; (b) status de CI confirmado; (c) cada item marcado como presente, parcial, ausente ou não verificado com caminho/arquivo; (d) issues deduplicadas e ordenadas por risco. Este documento é planejamento, não afirma que os testes passaram.
