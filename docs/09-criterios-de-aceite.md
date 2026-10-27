# 9. Critérios de aceite globais

## Produto

- [ ] A proposta e os três pilares estão claros no README.
- [ ] Uma pessoa consegue criar e executar um exemplo simples seguindo apenas a documentação.
- [ ] É possível usar Web sem habilitar IA.
- [ ] É possível usar o core sem importar a camada Web.
- [ ] Recursos experimentais estão claramente identificados.

## Arquitetura

- [ ] O core não depende de Axum nem de adaptadores de IA.
- [ ] Não existem ciclos de dependência entre crates.
- [ ] Os módulos e suas dependências são validados antes de iniciar o servidor.
- [ ] Eventos em memória têm garantias documentadas.
- [ ] Nenhum plugin nativo arbitrário é carregado implicitamente.

## API e DX

- [ ] Erros são tipados e respostas HTTP são consistentes.
- [ ] Configuração inválida gera erro legível antes do startup completo.
- [ ] Exemplos não usam `unwrap()` como tratamento de erro em código principal.
- [ ] Recursos opcionais podem ser habilitados e desabilitados.
- [ ] API pública e mudanças incompatíveis são revisadas por ADR.

## IA e automação

- [ ] Toda tool precisa ser registrada explicitamente.
- [ ] Toda execução tem limite de tempo e de passos.
- [ ] Falhas e cancelamentos são observáveis.
- [ ] O modo de teste usa provider falso e não exige chave real.
- [ ] Segredos e dados sensíveis são removidos de logs por padrão.

## Qualidade

- [ ] CI verifica formatação, lint, testes e exemplos.
- [ ] Feature flags importantes são testadas individualmente.
- [ ] Dependências são auditadas antes de releases.
- [ ] Benchmarks, quando divulgados, são reproduzíveis.
- [ ] Há instruções de contribuição, changelog e processo de release.
