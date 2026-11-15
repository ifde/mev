// ==============================
// Точка входа
// ==============================

// Модуль - либо папка, либо файл (по сути, кусок кода)
// мы их подключаем, чтобы Rust не собирал все файлы.
// подключение = означает, что становятся доступны все структуры / функции внутри модуля
mod build_plan;
mod cache;
mod chains;
mod cli; // подключить src/cli.rs
mod commands; // подключить src/commands/mod.rs
mod constants;
mod execution;
mod gateways;
mod pools;
mod find_proxy_pool;
mod find_proxy_transactions;
mod find_reference_pools;
mod route;
mod select_routes;
mod v4;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command}; // crate это текущий контекст проекта 

#[tokio::main] // tokio переписывает функцию, создавая менеджера, который может выполнять вызовы параллельно: runtime = tokio::runtime::Runtime::new().unwrap();
// если внутри написан отдельный последовательный код, то никакой многопоточности не будет
// async fn - превращает функцию в таску, которая выполняется только при вызове await. На моменте await нужно ждать, пока таска завершится. Это валидно только для каких-то HTTP вызовов, когда tokio может положить в текущий поток другую задачу, пока эта выполняется
// соответственно, внешняя функция тоже должна быть async
// нам нужно создавать async, потому что многие библиотеки его используют под капотом. Плюс tokio:: часто используется
async fn main() -> Result<()> {
    dotenv::dotenv().ok(); // загружаем переменные из .env или системных переменных окружения 
    let cli = Cli::parse();

    match cli.command {
        Command::CheckRpc(args) => commands::check_rpc::run(args).await, // await применятся к объекту Future, который создает tokio
        Command::InspectProxyPool(args) => commands::inspect_proxy_pool::run(args).await,
        Command::FindProxyPool(args) => commands::find_proxy_pool::run(args).await,
        Command::ListProxyTransactions(args) => commands::list_proxy_transactions::run(args).await,
        Command::FindReferencePools(args) => commands::find_reference_pools::run(args).await,
        Command::FindTwoPoolRoutes(args) => commands::find_two_pool_routes::run(args).await,
        Command::BuildPlan(args) => commands::build_plan::run(args).await,
        Command::QuoteTwoPoolCycle(args) => commands::quote_two_pool_cycle::run(args).await,
        Command::ExecuteTransaction(args) => commands::execute_transaction::run(args).await,
        Command::ExecutePlan(args) => commands::execute_plan::run(args).await,
        Command::SelectRoutes(args) => commands::select_routes::run(args),
    }
}
