# 10. Referências técnicas

As referências abaixo ajudam a orientar a arquitetura. O framework não deve copiar APIs nem alegar afiliação aos projetos citados. Verifique versões, licenças, MSRV e advisories novamente ao iniciar a implementação.

## Web e middleware

- **Axum — documentação no docs.rs:** https://docs.rs/axum/latest/axum/  
  Referência para roteamento, handlers, extractors, erros e integração com Tower. A documentação descreve Axum como uma biblioteca de routing e request handling integrada ao ecossistema Tower.
- **Actix Web — documentação oficial:** https://actix.rs/docs/  
  Referência adicional para desenho de framework web em Rust.
- **Tower — crate no crates.io:** https://crates.io/crates/tower  
  Referência para o modelo de serviços e composição de middleware.
- **Tokio — documentação:** https://docs.rs/tokio/latest/tokio/  
  Runtime e primitives de execução assíncrona.

## IA e integração de modelos

- **async-openai — documentação:** https://docs.rs/async-openai/latest/async_openai/  
  Exemplo de integração Rust com APIs de modelos e recursos opcionais por feature. É uma biblioteca não oficial para APIs da OpenAI; tratar como adaptador, não como dependência obrigatória do core.

## Linguagem e ferramentas

- **Rust Edition Guide:** https://doc.rust-lang.org/edition-guide/
- **The Rust Book:** https://doc.rust-lang.org/book/
- **Cargo Book:** https://doc.rust-lang.org/cargo/

## Orientação de uso

- Usar referências como inspiração e infraestrutura, não como justificativa para copiar código sem revisar licença e atribuição.
- As versões observadas nas páginas podem mudar. No início da implementação, confirmar versões estáveis mais recentes, compatibilidade entre crates e política de Rust Support.
- Registrar decisões de arquitetura no repositório e reavaliá-las com protótipos e testes.
