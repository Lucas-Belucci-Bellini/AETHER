# 6. Plano de testes e qualidade

## Pirâmide de testes

### Unitários
- Validação e mesclagem de configuração.
- Tipos e conversões de erro.
- Registro e dependências de módulos.
- Políticas de autorização de tools.
- Transições de estado de tarefas.

### Integração
- Subir aplicação de teste e testar endpoints.
- Validar status, headers, JSON e erros.
- Inicialização em falha de módulo e cleanup dos módulos que já iniciaram.
- Eventos com múltiplos subscribers e falhas em handlers.
- Cancelamento, timeout e limites de concorrência de tarefas.
- Provider falso que simula respostas, rate limit, timeout e falhas.

### Contratos e compatibilidade
- Testes dos exemplos públicos.
- `cargo test --workspace --all-targets`.
- Matriz de features (mínimo, web, modules, ai e full).
- MSRV e stable atual.
- Testes para garantir que crates opcionais não são ativadas pelo core sem necessidade.

### Segurança
- Limites de payload e inputs grandes.
- Sanitização/redaction de logs.
- Tentativas de usar tools não registradas ou não autorizadas.
- Limites máximos de passos, retries, tempo e tokens/custo quando o provedor permitir medir.
- Testes para prompt injection não conceder permissões adicionais à ferramenta.

## CI ativo e evidências atuais

O workflow `Rust CI` em `.github/workflows/ci.yml` executa formatação, verificação do workspace, testes com todas as features, Clippy com avisos tratados como erros e validação da MSRV 1.94. O workflow `Dependency Security Audit` executa `cargo audit` sobre o `Cargo.lock`.

Os testes públicos de lifecycle em `crates/liga-modules/tests/lifecycle_public_contract.rs` verificam:
- rollback apenas dos módulos que iniciaram com sucesso quando um módulo posterior falha;
- reporte de timeout no shutdown sem deixar de executar os hooks restantes.

Resultados de workflow devem sempre ser consultados para o SHA exato da alteração. A existência do workflow não significa que toda execução futura passou.

## Política para benchmarks

Não prometer “mais rápido que Axum/Actix” sem medições reproduzíveis. Definir cenários e ambiente, por exemplo:
- endpoint JSON simples;
- rota com middleware e validação;
- inicialização de aplicação com N módulos;
- throughput e latência p50/p95/p99;
- memória de processo em repouso e sob carga.

Registrar CPU, RAM, sistema operacional, versão do Rust, configuração e commit. Comparar com a implementação equivalente usando bibliotecas-base, não com cenários diferentes.

## Definição geral de pronto

Uma mudança só está pronta quando tem testes adequados, documentação, erros tratados e não amplia o escopo público sem uma decisão registrada.
