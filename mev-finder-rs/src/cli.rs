// ==============================
// Модуль для работы с CLI 
// ============================== 

/*  ==============================
Команда `mev-finder-rs check-rpc --rpc-url hello.com --analysis-block 24000000` 
Превращается в 

Cli {
    command: Command::CheckRpc(
        CheckRpcArgs {
            rpc_url: "https://...".to_owned(),
            analysis_block: 24_000_000,
        }
    )
}
// ============================== */ 

use alloy::primitives::{Address, B256}; 
use clap::{Args, Parser, Subcommand};

#[derive(Parser)] // для структуры Cli делаем реализацию трейта Parser. После этого появляется метод Cli::parse() 
// derive - встроенный механизм Rust 
#[command(name = "mev-finder-rs")] // задаем имя программы. command - аннотация библиотеки clap. 
pub struct Cli { // это структура для хранения CLI ввода 
    #[command(subcommand)] // поле command содержит подкоманду 
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    CheckRpc(CheckRpcArgs), 
    InspectProxyPool(InspectProxyPoolArgs), 
    FindProxyPool(FindProxyPoolArgs), 
    ListProxySwaps(ListProxySwapsArgs), 
    FindReferencePools(FindReferencePoolsArgs), 
    QuoteTwoPoolCycle(QuoteTwoPoolCycleArgs), 
}

#[derive(Args)]
pub struct CheckRpcArgs {
    #[arg(long, env = "MAINNET_RPC_URL")]
    pub rpc_url: String,

    #[arg(long)]
    pub analysis_block: u64, 

    #[arg(long, env = "V4_POOL_MANAGER")] // официальный адрес v4 PoolManager 
    pub pool_manager: Address, 
}  

#[derive(Args)]
pub struct InspectProxyPoolArgs {
    #[arg(long, env = "MAINNET_RPC_URL")]
    pub rpc_url: String,

    #[arg(long, env = "V4_POOL_MANAGER")]
    pub pool_manager: Address,

    #[arg(long)]
    pub pool_id: B256, // ровно 32 байта (256 бит). Удобно использовать для хешей 
                       // В Uniswap V4: PoolId = keccak256(abi.encode(PoolKey)) 

    #[arg(long)]
    pub from_block: u64,

    #[arg(long)]
    pub to_block: u64,
} 

#[derive(Args)]
pub struct FindProxyPoolArgs {
    #[arg(long)]
    pub token_a: Address,

    #[arg(long)]
    pub token_b: Address,
} 

#[derive(Args)]
pub struct ListProxySwapsArgs {
    #[arg(long, env = "MAINNET_RPC_URL")]
    pub rpc_url: String,

    #[arg(long, env = "V4_POOL_MANAGER")]
    pub pool_manager: Address,

    #[arg(long)]
    pub token_a: Address,

    #[arg(long)]
    pub token_b: Address,

    #[arg(long)]
    pub from_block: u64,

    #[arg(long)]
    pub to_block: u64,
} 

#[derive(Args)]
pub struct FindReferencePoolsArgs {
    #[arg(long, env = "THE_GRAPH_SUBGRAPH_URL_V4_ETH_MAINNET")]
    pub subgraph_url: String,

    #[arg(long, env = "THE_GRAPH_API_KEY")]
    pub api_key: String,

    #[arg(long, env = "V4_POOL_MANAGER")]
    pub pool_manager: Address,

    #[arg(long)]
    pub token_a: Address,

    #[arg(long)]
    pub token_b: Address, 

    #[arg(long, env = "MAINNET_RPC_URL")]
    pub rpc_url: String,

    #[arg(long, env = "V4_STATE_VIEW_ETH_MAINNET")]
    pub state_view: Address,

    #[arg(long)]
    pub analysis_block: u64,
} 

#[derive(Args)]
pub struct QuoteTwoPoolCycleArgs {
    #[command(flatten)]
    pub reference_search: FindReferencePoolsArgs, // передаем те же аргументы, что и для FindReferncePools 

    #[arg(long, env = "V4_QUOTER_ETH_MAINNET")]
    pub quoter: Address,

    #[arg(long)]
    pub start_currency: Address,

    #[arg(long)]
    pub amount_in: u128,
}