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

## CI inicial

1. Formatação: `cargo fmt --all -- --check`.
2. Lints: `cargo clippy --workspace --all-targets --all-features -- -D warnings` (revisar se features incompatíveis tornam `all-features` inviável).
3. Testes: workspace e exemplos.
4. Auditoria de dependências com ferramenta mantida e política definida pelo projeto.
5. Verificação de documentação e exemplos.
6. Teste de build nas plataformas realmente suportadas.

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
