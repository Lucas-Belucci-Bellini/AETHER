# Backlog inicial de issues

As issues abaixo são propostas para copiar para o rastreador do projeto. Recomenda-se concluir a fase anterior antes de ampliar o escopo.

## Épico 1 — Fundação

### Issue: Criar workspace inicial
**Objetivo:** criar o workspace Cargo e as crates mínimas previstas para a Fase 0.  
**Checklist:**
- [ ] Definir versão mínima do Rust e edição.
- [ ] Criar workspace e resolver dependências compartilhadas.
- [ ] Adicionar licença e política de contribuição provisórias.
- [ ] Garantir `cargo check --workspace` em clone limpo.
- [ ] Adicionar CI para fmt, clippy e testes.

### Issue: Registrar ADRs iniciais
**Objetivo:** revisar e aprovar as decisões propostas antes de estabilizar APIs.  
**Checklist:**
- [ ] Revisar stack HTTP.
- [ ] Revisar política de features.
- [ ] Revisar configuração e precedência.
- [ ] Revisar eventos e lifecycle.
- [ ] Registrar licença, MSRV e semver.

## Épico 2 — Core e configuração

### Issue: Implementar configuração tipada
**Objetivo:** carregar configuração de defaults, arquivo e ambiente com precedência documentada.  
**Checklist:**
- [ ] Erros de parsing legíveis.
- [ ] Validação antes do startup.
- [ ] Testar valores ausentes e inválidos.
- [ ] Garantir que segredos não sejam serializados em logs.

### Issue: Definir erro comum e AppContext
**Objetivo:** compartilhar contexto e erros sem acoplar o core à camada Web.  
**Checklist:**
- [ ] Erros públicos documentados.
- [ ] IDs de request/execução.
- [ ] Testes de conversão e propagação.

## Épico 3 — Web MVP

### Issue: Criar servidor HTTP e roteamento
**Objetivo:** expor rotas GET/POST e estado de aplicação usando Axum como infraestrutura inicial.  
**Checklist:**
- [ ] Exemplo `hello-api`.
- [ ] Middleware de tracing e timeout.
- [ ] Erros de entrada e 404 testados.
- [ ] README executável em clone limpo.

### Issue: Padronizar erros HTTP
**Objetivo:** converter erros internos em respostas seguras e consistentes.  
**Checklist:**
- [ ] Estrutura JSON de erro.
- [ ] Sem stack trace para o cliente.
- [ ] Correlation ID em logs.
- [ ] Testes para erros esperados e inesperados.

## Épico 4 — Modularidade

### Issue: Criar contrato Module e lifecycle
**Objetivo:** inicializar e encerrar módulos em ordem validada.  
**Checklist:**
- [ ] Registro explícito.
- [ ] Dependência ausente detectada.
- [ ] Ciclos e nomes duplicados detectados.
- [ ] Shutdown reverso testado.

### Issue: Implementar event bus em memória
**Objetivo:** permitir publicação e inscrição em eventos locais com semântica definida.  
**Checklist:**
- [ ] Definir concorrência e ordenação.
- [ ] Definir comportamento para erro de subscriber.
- [ ] Testar cancelamento e subscribers múltiplos.
- [ ] Documentar ausência de durabilidade.

## Épico 5 — IA e automação

### Issue: Implementar task runner local
**Objetivo:** executar tarefas async com cancelamento, timeout e limite de concorrência.  
**Checklist:**
- [ ] Estados de tarefa.
- [ ] Timeout e cancelamento testados.
- [ ] Retry limitado e apenas para erros elegíveis.
- [ ] Métricas e tracing básicos.

### Issue: Definir traits ModelProvider e Tool
**Objetivo:** criar contratos neutros de provedor e ferramenta.  
**Checklist:**
- [ ] Provider falso para testes.
- [ ] Schema e validação de argumentos.
- [ ] Permissões deny-by-default.
- [ ] Limites de passos e prazo.

### Issue: Criar adaptador opcional para um provedor
**Objetivo:** demonstrar um caminho real sem tornar o adaptador obrigatório.  
**Checklist:**
- [ ] Credenciais somente via ambiente/secret manager.
- [ ] Feature separada.
- [ ] Timeout e retry limitados.
- [ ] Teste real opt-in; testes de CI usam mocks.

## Épico 6 — Release

### Issue: Preparar release candidata
**Checklist:**
- [ ] Rodar todos os exemplos em CI.
- [ ] Auditar dependências e licenças.
- [ ] Revisar a superfície de API pública.
- [ ] Criar changelog e instruções de upgrade.
- [ ] Publicar limites conhecidos e benchmarks reproduzíveis.
