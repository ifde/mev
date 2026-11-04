use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    sol_types::SolEvent,
};
use anyhow::{bail, Context, Result};

use crate::{
    cli::InspectProxyPoolArgs,
    v4::Initialize,
};

pub async fn run(args: InspectProxyPoolArgs) -> Result<()> {
    if args.from_block > args.to_block {
        bail!("from_block must not be greater than to_block");
    }

    let rpc_url = args
        .rpc_url
        .parse()
        .context("rpc_url must be a valid HTTP URL")?;

    let provider = ProviderBuilder::new().connect_http(rpc_url);

    let filter = Filter::new()
        .address(args.pool_manager)
        .event_signature(Initialize::SIGNATURE_HASH) // идентификатор события Initialize 
                                                // keccak256("Initialize(bytes32,address,address,uint24,int24,address,uint160,int24)")
        .topic1(args.pool_id)
        .from_block(args.from_block)
        .to_block(args.to_block);

    let logs = provider
        .get_logs(&filter) // RPC-запрос eth_getLogs 
        .await
        .context("eth_getLogs for Initialize failed")?;

    if logs.len() != 1 {
        bail!(
            "expected exactly one Initialize log for pool_id, found {}",
            logs.len()
        );
    }

    let decoded = logs[0] // возвращает сырые логи 
        .log_decode_validate::<Initialize>() // расщифровываем лог по ABI-схеме события Initialize 
                                            // ABI = Application Binary Interface - desciption of what a contract / event does
                                            // It is compiled to raw bytes so EVM can understand what to execute. 
        .context("failed to decode Initialize log")?; 
    let event = decoded.data(); 

    if event.hooks != Address::ZERO {
        bail!("proxy pool must be hookless, got hooks={:?}", event.hooks);
    }

    println!("pool_id={:?}", event.id);
    println!("currency0={:?}", event.currency0);
    println!("currency1={:?}", event.currency1);
    println!("fee={:?}", event.fee);
    println!("tick_spacing={:?}", event.tickSpacing);
    println!("hooks={:?}", event.hooks);
    println!("sqrt_price_x96={:?}", event.sqrtPriceX96);
    println!("initial_tick={:?}", event.tick);

    Ok(())
}