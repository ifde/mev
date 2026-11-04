// ==============================
// сохраняет вектор свопов 
// ============================== 

use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    sol_types::SolEvent,
};
use anyhow::{bail, Context, Result};

use crate::{
    proxy_pool::{ProxyPool, ProxySwap},
    v4::Swap, 
    constants::{MAX_LOG_BLOCKS, MAX_SWAPS}
}; 


pub struct LoadProxySwapsRequest {
    pub rpc_url: String,
    pub pool_manager: Address,
    pub proxy_pool: ProxyPool,
    pub from_block: u64,
    pub to_block: u64,
}

pub struct LoadedProxySwaps {
    pub found_log_count: usize,
    pub swaps: Vec<ProxySwap>,
}

pub async fn load_proxy_swaps(
    request: LoadProxySwapsRequest,
) -> Result<LoadedProxySwaps> {
    if request.from_block > request.to_block {
        bail!("from_block must not be greater than to_block");
    }

    if request.to_block - request.from_block + 1 > MAX_LOG_BLOCKS {
        bail!("block range must not exceed {MAX_LOG_BLOCKS} blocks");
    }

    let rpc_url = request
        .rpc_url
        .parse()
        .context("rpc_url must be a valid HTTP URL")?;

    let provider = ProviderBuilder::new().connect_http(rpc_url);

    let chain_id = provider
        .get_chain_id()
        .await
        .context("eth_chainId request failed")?;

    if chain_id != 1 {
        bail!("expected Ethereum Mainnet chain_id=1, got {chain_id}");
    }

    let filter = Filter::new()
        .address(request.pool_manager)
        .event_signature(Swap::SIGNATURE_HASH)
        .topic1(request.proxy_pool.pool_id)
        .from_block(request.from_block)
        .to_block(request.to_block);

    let mut logs = provider
        .get_logs(&filter)
        .await
        .context("eth_getLogs for Swap failed")?;

    logs.sort_by(|left, right| { // сортируем логи
        (left.block_number, left.transaction_index, left.log_index,)
        .cmp(&(right.block_number, right.transaction_index, right.log_index,)) // функция сравнения 
        // возвращает Ordering::Less, Ordering::Equal, Ordering::Greater
    }); 

    let found_log_count = logs.len();

    logs.truncate(MAX_SWAPS);

    let mut swaps = Vec::with_capacity(logs.len());

    for log in logs {
        let block_number = log
            .block_number
            .context("Swap log has no block number")?; 

        let transaction_index = log
            .transaction_index
            .context("Swap log has no transaction index")?;

        let transaction_hash = log
            .transaction_hash
            .context("Swap log has no transaction hash")?;

        let log_index = log
            .log_index
            .context("Swap log has no log index")?;

        let decoded = log
            .log_decode_validate::<Swap>()
            .context("failed to decode Swap log")?;

        let event = decoded.data();

        if event.id != request.proxy_pool.pool_id {
            bail!("Swap log pool_id does not match the selected proxy pool");
        }

        let amount0 = i128::try_from(event.amount0)
            .context("amount0 does not fit into i128")?;

        let amount1 = i128::try_from(event.amount1)
            .context("amount1 does not fit into i128")?;

        let proxy_swap = if amount0 < 0 && amount1 > 0 {
            ProxySwap {
                block_number, 
                transaction_index, 
                transaction_hash,
                log_index,
                zero_for_one: true,
                amount_in: amount0.unsigned_abs(),
            }
        } else if amount0 > 0 && amount1 < 0 {
            ProxySwap {
                block_number, 
                transaction_index, 
                transaction_hash,
                log_index,
                zero_for_one: false,
                amount_in: amount1.unsigned_abs(),
            }
        } else {
            bail!("invalid Swap deltas: amount0={amount0}, amount1={amount1}");
        };

        swaps.push(proxy_swap);
    }

    Ok(LoadedProxySwaps {
        found_log_count,
        swaps,
    })
}