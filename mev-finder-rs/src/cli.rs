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

#[derive(Parser)]
// для структуры Cli делаем реализацию трейта Parser. После этого появляется метод Cli::parse()
// derive - встроенный механизм Rust
#[command(name = "mev-finder-rs")] // задаем имя программы. command - аннотация библиотеки clap. 
pub struct Cli {
    // это структура для хранения CLI ввода
    #[command(subcommand)] // поле command содержит подкоманду
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    CheckRpc(CheckRpcArgs),
    InspectProxyPool(InspectProxyPoolArgs),
    FindProxyPool(FindProxyPoolArgs),
    ListProxyTransactions(ListProxyTransactionsArgs),
    FindReferencePools(FindReferencePoolsArgs),
    FindTwoPoolRoutes(FindTwoPoolRoutesArgs),
    BuildPlan(BuildPlanArgs),
    QuoteTwoPoolCycle(QuoteTwoPoolCycleArgs),
    ExecuteTransaction(ExecuteTransactionArgs),
    ExecutePlan(ExecutePlanArgs),
    SelectRoutes(SelectRoutesArgs),
    Backtest(BacktestArgs),
}

#[derive(Args, serde::Serialize)]
pub struct BacktestArgs {
    #[arg(long)]
    pub run_dir: std::path::PathBuf,
    #[arg(long)]
    pub end_block: u64,
    #[arg(long, default_value_t = usize::MAX)]
    pub max_transactions: usize,
    #[arg(long, default_value_t = 1)]
    pub route_limit: usize,
    #[arg(long, default_value_t = 500_000)]
    pub attempt_gas: u64,
    /// Сколько ликвидности добавляем в таргет пул при деплое. По умолчанию - вся ликвидность прокси пула на блоке analysis_block. 
    #[arg(long)]
    pub liquidity: Option<u128>,
    #[arg(long, allow_hyphen_values = true)]
    pub tick_lower: Option<i32>,
    #[arg(long, allow_hyphen_values = true)]
    pub tick_upper: Option<i32>,
    #[arg(long, default_value_t = 0)]
    // комиссия таргет пула 
    pub target_fee: u32,
    #[arg(long, default_value_t = 300)] 
    // комиссия бейзлайн пула (с которым будем сравнивать, по умолчанию - 0)  
    pub baseline_fee: u32,
    #[arg(long, env = "MAINNET_RPC_URL", hide_env_values = true)]
    #[serde(skip)]
    pub rpc_url: String,
}

#[derive(Args)]
pub struct ExecuteTransactionArgs {
    #[arg(long)]
    pub plan: std::path::PathBuf,
    #[arg(long, default_value_t = 0)]
    pub transaction_plan_index: usize,
    #[arg(long, env = "MAINNET_RPC_URL", hide_env_values = true)]
    pub rpc_url: String,
    #[arg(long, env = "V4_POOL_MANAGER")]
    pub pool_manager: Address,
    #[arg(long, env = "V4_STATE_VIEW_ETH_MAINNET")]
    pub state_view: Address,
    #[arg(long, env = "V4_QUOTER_ETH_MAINNET")]
    pub quoter: Address,
    #[arg(
        long,
        default_value = "../mev-foundry/out/ReplaySwap.sol/ReplaySwap.json"
    )]
    pub swap_artifact: std::path::PathBuf,
    #[arg(long, default_value = "evaluation-result.json")]
    pub output: std::path::PathBuf,
}

#[derive(Args)]
pub struct ExecutePlanArgs {
    #[arg(long)]
    pub plan: std::path::PathBuf,
    #[arg(long)]
    pub output_dir: std::path::PathBuf,
    #[arg(long, env = "MAINNET_RPC_URL", hide_env_values = true)]
    pub rpc_url: String,
    #[arg(long, env = "V4_POOL_MANAGER")]
    pub pool_manager: Address,
    #[arg(long, env = "V4_STATE_VIEW_ETH_MAINNET")]
    pub state_view: Address,
    #[arg(long, env = "V4_QUOTER_ETH_MAINNET")]
    pub quoter: Address,
    #[arg(
        long,
        default_value = "../mev-foundry/out/ReplaySwap.sol/ReplaySwap.json"
    )]
    pub swap_artifact: std::path::PathBuf,
}

#[derive(Args)]
pub struct SelectRoutesArgs {
    /// Каталог одного запуска: содержит results/ и получит selected-routes.json.
    #[arg(long)]
    pub run_dir: std::path::PathBuf,
}

#[derive(Args)]
pub struct CheckRpcArgs {
    #[arg(long, env = "MAINNET_RPC_URL", hide_env_values = true)]
    pub rpc_url: String,

    #[arg(long)]
    pub analysis_block: u64,

    #[arg(long, env = "V4_POOL_MANAGER")] // официальный адрес v4 PoolManager
    pub pool_manager: Address,
}

#[derive(Args)]
pub struct InspectProxyPoolArgs {
    #[arg(long, env = "MAINNET_RPC_URL", hide_env_values = true)]
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
    #[arg(long, env = "THE_GRAPH_SUBGRAPH_URL_V4_ETH_MAINNET")]
    pub subgraph_url: String,

    #[arg(long, env = "THE_GRAPH_API_KEY", hide_env_values = true)]
    pub api_key: String,

    #[arg(long, env = "MAINNET_RPC_URL", hide_env_values = true)]
    pub rpc_url: String,

    #[arg(long, env = "V4_STATE_VIEW_ETH_MAINNET")]
    pub state_view: Address,

    #[arg(long)]
    pub analysis_block: u64,

    #[arg(long)]
    pub token_a: Address,

    #[arg(long)]
    pub token_b: Address,
}

#[derive(Args)]
pub struct ListProxyTransactionsArgs {
    #[arg(long, env = "THE_GRAPH_SUBGRAPH_URL_V4_ETH_MAINNET")]
    pub subgraph_url: String,

    #[arg(long, env = "THE_GRAPH_API_KEY", hide_env_values = true)]
    pub api_key: String,

    #[arg(long, env = "MAINNET_RPC_URL", hide_env_values = true)]
    pub rpc_url: String,

    #[arg(long, env = "V4_POOL_MANAGER")]
    pub pool_manager: Address,

    #[arg(long)]
    pub token_a: Address,

    #[arg(long)]
    pub token_b: Address,

    #[arg(long, env = "V4_STATE_VIEW_ETH_MAINNET")]
    pub state_view: Address,

    #[arg(long)]
    pub analysis_block: u64,

    #[arg(long)]
    pub from_block: u64,

    #[arg(long)]
    pub to_block: u64,
}

#[derive(Args)]
pub struct FindReferencePoolsArgs {
    #[arg(long, env = "THE_GRAPH_SUBGRAPH_URL_V4_ETH_MAINNET")]
    pub subgraph_url: String,

    #[arg(long, env = "THE_GRAPH_API_KEY", hide_env_values = true)]
    pub api_key: String,

    #[arg(long)]
    pub token_a: Address,

    #[arg(long)]
    pub token_b: Address,

    #[arg(long, env = "MAINNET_RPC_URL", hide_env_values = true)]
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

    /// RPC Anvil для проверки на latest. Поиск пулов использует основной RPC.
    #[arg(long)]
    pub anvil_rpc_url: Option<String>,

    #[arg(long, env = "V4_QUOTER_ETH_MAINNET")]
    pub quoter: Address,

    #[arg(long)]
    pub start_currency: Address,

    #[arg(long)]
    pub amount_in: u128,
}

#[derive(Args)]
pub struct FindTwoPoolRoutesArgs {
    #[command(flatten)]
    pub reference_search: FindReferencePoolsArgs,
}

#[derive(Args)]
pub struct BuildPlanArgs {
    #[arg(long, env = "THE_GRAPH_SUBGRAPH_URL_V4_ETH_MAINNET")]
    pub subgraph_url: String,

    #[arg(long, env = "THE_GRAPH_API_KEY", hide_env_values = true)]
    pub api_key: String,

    #[arg(long, env = "MAINNET_RPC_URL", hide_env_values = true)]
    pub rpc_url: String,

    #[arg(long, env = "V4_STATE_VIEW_ETH_MAINNET")]
    pub state_view: Address,

    #[arg(long)]
    pub token_a: Address,

    #[arg(long)]
    pub token_b: Address,

    /// Не выбирать автоматически: использовать этот hookless proxy-пул.
    #[arg(long)]
    pub proxy_pool_id: Option<B256>,

    #[arg(long, env = "V4_POOL_MANAGER")]
    pub pool_manager: Address,

    #[arg(long)]
    pub from_block: u64,

    #[arg(long)]
    pub to_block: u64,

    #[arg(long, default_value_t = 100)]
    pub max_proxy_transactions: usize,

    #[arg(long, default_value = "output.json")]
    pub output: String,
}
