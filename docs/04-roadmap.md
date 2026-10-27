# 4. Roadmap por fases

O roadmap é orientado a entregas e critérios de aceite, não a datas fixas. Avançar somente quando a fase atual estiver demonstrável e coberta por testes.

## Fase 0 — Fundação e decisões

**Entregas**
- Nome do projeto e licença escolhidos.
- Objetivos, não objetivos e público-alvo documentados.
- Workspace vazio que compila.
- ADRs para runtime, camada HTTP, features, eventos e política de plugins.
- CI com `cargo fmt --check`, `cargo clippy` e `cargo test`.

**Pronto quando**
- Um clone limpo compila e passa no CI.
- A política de versão mínima do Rust e de dependências está escrita.

## Fase 1 — Core

**Entregas**
- Erros tipados comuns.
- Configuração tipada e validação em startup.
- Contexto da aplicação.
- Contratos básicos de lifecycle.
- Exemplo mínimo sem servidor HTTP.

**Pronto quando**
- Configuração inválida falha antes de iniciar recursos.
- Testes confirmam inicialização e shutdown, inclusive em falhas intermediárias.

## Fase 2 — Web MVP

**Entregas**
- Aplicação HTTP com rotas e handlers.
- Estado compartilhado.
- Middleware para tracing, timeout e limites básicos.
- Respostas de erro consistentes.
- Exemplo CRUD simples em memória.
- Testes de integração sem exigir abrir porta pública real, quando possível.

**Pronto quando**
- Exemplo executa localmente a partir do README.
- Rotas de sucesso, validação, não encontrado e erro interno estão testadas.
- Segredos e detalhes internos não aparecem nas respostas públicas.

## Fase 3 — Módulos e eventos

**Entregas**
- Trait `Module` e registro explícito.
- Validação de dependências e ordem de inicialização.
- Hooks de startup/shutdown.
- Barramento de eventos em memória.
- Registro de rotas e configuração por módulo.

**Pronto quando**
- Testes cobrem módulo duplicado, dependência ausente, ciclo e falha de inicialização.
- A ordem de shutdown é definida e testada.
- As garantias e limitações dos eventos estão documentadas.

## Fase 4 — Automação e IA

**Entregas**
- Runner local de tarefas async com cancelamento.
- Estados de tarefa, timeout, concorrência limitada e política de retry.
- Traits de provider e tools.
- Primeiro adaptador de modelo opcional.
- Agente de exemplo que chama uma ferramenta permitida.
- Logs/traces para cada execução.

**Pronto quando**
- Testes não dependem de um endpoint real de IA: provider falso cobre o fluxo principal.
- A execução termina ao atingir limite de passos ou prazo.
- Ferramentas sensíveis exigem autorização explícita.
- Nenhum segredo aparece em logs ou erros.

## Fase 5 — Prontidão para produção

**Entregas**
- Benchmarks reproduzíveis e baseline publicada.
- Guia de deploy e configuração.
- Revisão de segurança e dependências.
- Documentação da API pública.
- Política de estabilidade e compatibilidade.
- Processo de releases e changelog.

**Pronto quando**
- Os exemplos oficiais passam em CI.
- Há checklist de release, procedimento de rollback e política de divulgação de vulnerabilidades.
- Os limites conhecidos estão publicados.

## Fase futura — Recursos distribuídos

Avaliar somente após uso real: fila durável, scheduler persistente, event store, execução distribuída, adaptadores WASM para extensões e múltiplos provedores de IA. Essas escolhas mudam muito o escopo e não devem bloquear o primeiro release.
