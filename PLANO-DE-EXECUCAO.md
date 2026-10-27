# Plano de execução resumido

## Ordem recomendada

1. **Definir nome, licença, MSRV e política de versão.** Nome definido: **Liga** (ADR-0008); MSRV 1.94 (ADR-0009).
2. **Criar workspace e CI.** O repositório deve compilar antes de incluir funcionalidades.
3. **Implementar core e configuração.** Primeiro definir contratos; depois ligar serviços reais.
4. **Entregar Web MVP.** Um servidor simples, rotas, erros, middleware e testes.
5. **Adicionar módulos e eventos.** Validar lifecycle e dependências antes de iniciar a aplicação.
6. **Adicionar automação.** Tarefas com cancelamento, timeout e concorrência limitada.
7. **Adicionar IA.** Provider e tools com permissões explícitas, mock tests e um adaptador opcional.
8. **Preparar release.** Segurança, documentação, compatibilidade, benchmark e changelog.

## O que não fazer agora

- Não criar servidor HTTP, executor assíncrono ou cliente de IA do zero sem motivo comprovado.
- Não tentar criar sistema de plugins nativos dinâmicos na primeira versão.
- Não colocar todos os recursos numa única crate com dependências obrigatórias.
- Não prometer superioridade de desempenho sem benchmark reproduzível.
- Não implementar filas distribuídas, cluster e persistência de agentes antes de provar o MVP.

## Primeiro marco concreto

O primeiro marco deve ser: **uma API de catálogo com duas rotas, configuração validada, middleware de tracing e timeout, erro padronizado e testes automatizados**. A estrutura deve permitir que essa API seja organizada como um módulo, mas a integração de IA fica para uma fase posterior.

## Checklist do repositório inicial

- [ ] Criar repositório (local pronto; aguardando liberação do GitHub).
- [x] Escolher nome definitivo: Liga.
- [x] Adicionar esta documentação revisada.
- [x] Aprovar ADR-0001 e ADR-0002 (e ADR-0003 a 0012).
- [x] Criar workspace mínimo.
- [ ] Configurar CI no GitHub (bloqueado pelo congelamento; o equivalente local é `scripts/verify.sh`).
- [x] Implementar `hello-api`.
- [ ] Abrir issues por fase e vincular critérios de aceite.

## Status por fase (2026-10-25)

| Fase | Status |
|---|---|
| 0: fundação | Concluída |
| 1: core | Concluída |
| 2: web MVP | Concluída |
| 3: módulos e eventos | Concluída |
| 4: automação e IA | Concluída, exceto o adaptador real de provedor |
| 5: produção | Pendente (benchmarks, guia de deploy, changelog, licença) |
