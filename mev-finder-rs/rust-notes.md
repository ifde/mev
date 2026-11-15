### Некоторые заметки по Rust 

#### bail 

`bail! (....)` - это макрос из anyhow 
(**Anyhow** - Rust-библиотека для удобной обработки ошибок в приложениях и CLI)

Делает то же самое, что 
```rust
return Err(anyhow::anyhow!(
    "no reference pools have active liquidity at analysis_block"
)); 
```  

Позволяет возвращать единый тип ошибок: `anyhow::Result<T> = Result<T, anyhow::Error>` (именно этот объект возвращается при `.context("StateView.getSlot0 failed")`)

Это не Panic, то есть она не аварийно завершает работу, а возвращает ошибку. 

#### #[sol(rpc)]

`#[sol(rpc)]` // Rust обертка (атрибут). 
Он передаёт макросу sol! параметр rpc

Чтобы вызывать Solidity-функцию как Rust-метод  

