# 7. Segurança e observabilidade

## Segurança Web

- Definir limites de corpo, requisições, conexões e concorrência.
- Configurar timeouts explícitos; evitar operações sem prazo em caminhos críticos.
- Não confiar em `Host`, origem, headers ou payloads fornecidos pelo cliente.
- CORS deve ser restritivo por padrão e configurado conscientemente.
- Autenticação/autorização devem ser pontos de extensão; não prometer um sistema completo de identidade no MVP.
- Nunca devolver stack trace, segredo, query interna ou caminho local em resposta pública.

## Segurança de IA e ferramentas

- Deny-by-default: ferramenta só executa se estiver registrada e autorizada.
- Não interpretar saída de modelo como autorização.
- Separar instruções do sistema, conteúdo externo e parâmetros da ferramenta.
- Validar schema, tamanho e tipos de argumentos antes de executar tools.
- Limitar passos, duração, concorrência, tamanho de entrada/saída e retries.
- Operações com efeitos externos exigem política explícita; para operações destrutivas, considerar confirmação humana.
- Segredos via ambiente ou secret manager; nunca inserir tokens em prompts, outputs ou logs.
- Usar provider mock nos testes; chamadas reais de modelo só em testes opt-in.

## Observabilidade

- Tracing estruturado com correlation/request IDs.
- Spans para requisições, inicialização de módulos, eventos, tarefas e chamadas de provider.
- Métricas para taxa de erros, latência, retries, tarefas pendentes e falhas de tools.
- Redaction de chaves, cookies, tokens, prompts privados e payloads sensíveis.
- Níveis de log configuráveis por ambiente.
- Evitar gravar todo prompt/resposta por padrão; conteúdo pode ser pessoal ou confidencial.

## Resiliência

- Política de retry apenas para erros transitórios e operações seguras para repetir.
- Backoff limitado e jitter para chamadas a provedores externos.
- Timeout, cancelamento e shutdown gracioso.
- Circuit breaker pode ser considerado após medir necessidade; não precisa entrar no core de início.
- Idempotência deve ser avaliada para jobs que possam ser retomados.

## Modelo de ameaça inicial

| Ativo | Risco | Controle inicial |
|---|---|---|
| API e dados dos usuários | abuso, payload excessivo, vazamento | limites, autenticação como integração, validação, logs seguros |
| Chaves de provedor | exfiltração | env/secret manager, redaction, nunca logar headers de auth |
| Ferramentas de IA | execução não autorizada | allowlist, schemas, políticas, timeout e auditoria |
| Plugins | código não confiável | sem carregamento nativo dinâmico no MVP; sandbox separado se necessário |
| Fila de tarefas | sobrecarga e repetição | limite de concorrência, retry limitado, cancelamento e métricas |
