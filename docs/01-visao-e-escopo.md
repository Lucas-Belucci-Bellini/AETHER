# 1. Visão, escopo e não objetivos

## Visão

Criar um framework de backend para Rust que ajude uma pessoa desenvolvedora a começar com uma API pequena e evoluir, sem reescrever a base, para um serviço modular com eventos, tarefas assíncronas e integrações de IA.

## Usuários-alvo

- Desenvolvedores Rust que criam APIs e serviços HTTP.
- Equipes que querem dividir aplicações em módulos bem definidos.
- Projetos internos, ferramentas de desenvolvimento e serviços de automação.
- Aplicações com agentes de IA que precisam chamar ferramentas com permissões e limites explícitos.

## Problema a resolver

Hoje é possível montar esses sistemas com bibliotecas especializadas, mas a equipe precisa decidir como integrar configuração, inicialização, observabilidade, erros, módulos e execução de tarefas. O framework deve padronizar essa composição sem esconder as bibliotecas subjacentes.

## Escopo funcional

### Web
- Servidor HTTP e roteamento.
- Handlers assíncronos e estado compartilhado.
- Middleware para tracing, CORS, limites, compressão e timeout.
- Erros de domínio convertidos para respostas HTTP padronizadas.
- Validação de entrada e configuração por ambiente.
- Documentação OpenAPI em uma etapa posterior do MVP, caso não comprometa a base.

### Modularidade
- Registro explícito de módulos.
- Hooks de inicialização e encerramento.
- Configuração por módulo.
- Barramento de eventos tipados ou payloads serializáveis, com semântica definida.
- Dependências entre módulos declaradas e validadas.
- Pontos de extensão sem obrigar o uso de bibliotecas de plugins dinâmicos.

### IA e automação
- Trait independente para provedores de modelos.
- Interface comum para ferramentas (tools) registradas.
- Execução de tarefas assíncronas com estado e cancelamento.
- Políticas de permissões, timeout, limite de chamadas e orçamento quando aplicável.
- Tracing de execução e registro de erros sem expor segredos.
- Adaptadores opcionais para provedores específicos.

## Fora do escopo inicial

- Framework frontend para renderizar interfaces web no navegador.
- Substituir Tokio, Hyper, Axum, Tower, SQLx ou bibliotecas de IA maduras.
- Sistema de plugins binários nativos carregados em runtime desde a versão inicial.
- Cluster distribuído, filas duráveis, scheduler distribuído e orquestração multiagente no MVP.
- ORM próprio, sistema de templates próprio ou linguagem de configuração própria.
- Promessa de compatibilidade com todos os provedores de IA desde o primeiro lançamento.

## Diferencial desejado

O diferencial não deve ser “fazer tudo do zero”. Deve ser a experiência integrada: convenções claras, dependências opcionais, lifecycle consistente, boas ferramentas de desenvolvimento e segurança previsível.

## Métricas a acompanhar

- Tempo para criar e executar uma API de exemplo a partir do template.
- Quantidade de dependências ativadas por configuração mínima.
- Cobertura de testes de integração para rotas e módulos.
- Latência e uso de memória medidos por benchmarks reproduzíveis.
- Número de decisões implícitas e falhas de configuração detectadas em startup.
- Qualidade da documentação: todos os exemplos principais testados no CI.
