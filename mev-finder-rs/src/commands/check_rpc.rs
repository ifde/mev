// ==============================
// Основная логика - Проверяем подключение к RPC 
// ============================== 

use alloy::{
    eips::{BlockId, BlockNumberOrTag},
    providers::{Provider, ProviderBuilder}, 
};
use anyhow::{bail, Context, Result};

use crate::cli::CheckRpcArgs;

pub async fn run(args: CheckRpcArgs) -> Result<()> {
    let rpc_url = args
        .rpc_url
        .parse() // возвращает Result<Url, Error>
        .context("rpc_url must be a valid HTTP URL")?; // если объект ошибки, то сразу делаем Err(error).context("rpc_url must be a valid HTTP URL");

    let provider = ProviderBuilder::new().connect_http(rpc_url); // подключаемся к RPC через Alloy 

    let chain_id = provider
        .get_chain_id()
        .await
        .context("eth_chainId request failed")?;

    if chain_id != 1 {
        bail!("expected Ethereum Mainnet chain_id=1, got {chain_id}");
    }

    let header = provider
        .get_header_by_number(BlockNumberOrTag::Number(args.analysis_block)) // получаем конкретный блок 
        .await
        .context("eth_getBlockByNumber request failed")? // распаковываем Future объект 
        .context("analysis_block was not found")?; // распаковывем результат функции get_header_by_number() 

    let pool_manager_code = provider
      .get_code_at(args.pool_manager) // готовит RPC запрос "верни байткод контракта по адресу pool_manager" 
      .block_id(BlockId::number(args.analysis_block)) // на каком блоке прочитать байткод 
      .await
      .context("eth_getCode for pool_manager failed")?;

    if pool_manager_code.is_empty() {
        bail!(
            "PoolManager has no bytecode at block {}",
            args.analysis_block
        );
    }

    println!("chain_id={}", chain_id); 
    println!("analysis_block={}", header.number);
    println!("block_hash={}", header.hash);
    println!("timestamp={}", header.timestamp);
    println!(
        "base_fee_per_gas={}",
        header
            .base_fee_per_gas // имеет тип struct Option<u64>: Some или None 
            .map(|value| value.to_string()) // // преобразуем u64 в &str 
            .unwrap_or_else(|| "none".to_owned()) // создаем String из &str 
    ); 
    println!("pool_manager={}", args.pool_manager); 
    println!("pool_manager_code_bytes={}", pool_manager_code.len()); 

    Ok(())
}