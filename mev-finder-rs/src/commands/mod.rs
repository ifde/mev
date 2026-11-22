pub mod check_rpc; // подключить модуль и сделать его публичным 
// (доступным для внешних импортеров - main.rs)
// то есть, теперь main.rs импортит и все эти подмодули
pub mod backtest;
pub mod build_plan;
pub mod execute_plan;
pub mod execute_transaction;
pub mod find_proxy_pool;
pub mod find_reference_pools;
pub mod find_two_pool_routes;
pub mod inspect_proxy_pool;
pub mod list_proxy_transactions;
pub mod quote_two_pool_cycle;
pub mod select_routes;
