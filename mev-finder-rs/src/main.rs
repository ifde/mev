// ==============================
// Точка входа 
// ============================== 

// Модуль - либо папка, либо файл (по сути, кусок кода) 
// мы их подключаем, чтобы Rust не собирал все файлы. 
// подключение = означает, что становятся доступны все структуры / функции внутри модуля
mod cli; // подключить src/cli.rs
mod commands; // подключить src/commands/mod.rs 
mod v4; 
mod proxy_pool; 
mod proxy_swaps; 
mod constants;  
mod reference_pool; 
mod subgraph;  
mod pool_key; 
mod state_view; 
mod route; 
mod reference_search; 
mod quoter; 

use anyhow::Result; 
use clap::Parser; 

use crate::cli::{Cli, Command}; // crate это текущий контекст проекта 



#[tokio::main] // tokio запускает async-задачи в отдельном рантайме и ждёт I/O, например HTTP
async fn main() -> Result<()> {
    dotenv::dotenv().ok(); // загружаем переменные из .env или системных переменных окружения 
    let cli = Cli::parse();

    match cli.command {
        Command::CheckRpc(args) => commands::check_rpc::run(args).await, // await применятся к объекту Future, который создает tokio 
        Command::InspectProxyPool(args) => commands::inspect_proxy_pool::run(args).await, 
        Command::FindProxyPool(args) => commands::find_proxy_pool::run(args), 
        Command::ListProxySwaps(args) => commands::list_proxy_swaps::run(args).await, 
        Command::FindReferencePools(args) => commands::find_reference_pools::run(args).await, 
        Command::QuoteTwoPoolCycle(args) =>  commands::quote_two_pool_cycle::run(args).await, 
    }
} 

