// ==============================
// Чтение через RPC Swap-событий конкретного пула, который зарегистрирован в PoolManger.
// ==============================

use alloy::{
    primitives::{Address, B256},
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    sol_types::SolEvent,
};
use anyhow::{Context, Result, bail};

use crate::v4::Swap;

#[derive(Clone, Copy, Debug)]
pub struct PoolSwapLog {
    pub block_number: u64,
    pub transaction_index: u64,
    pub transaction_hash: B256,
    pub log_index: u64,
    pub amount0: i128,
    pub amount1: i128,
}

pub async fn load_pool_swap_logs(
    rpc_url: &str,
    expected_chain_id: u64,
    pool_manager: Address,
    pool_id: B256,
    from_block: u64,
    to_block: u64,
) -> Result<Vec<PoolSwapLog>> {
    let rpc_url = rpc_url
        .parse()
        .context("rpc_url must be a valid HTTP URL")?;

    let provider = ProviderBuilder::new().connect_http(rpc_url);

    let chain_id = provider
        .get_chain_id()
        .await
        .context("eth_chainId request failed")?;

    if chain_id != expected_chain_id {
        bail!("expected chain_id={expected_chain_id}, got {chain_id}");
    }

    let filter = Filter::new()
        .address(pool_manager)
        .event_signature(Swap::SIGNATURE_HASH)
        .topic1(pool_id)
        .from_block(from_block)
        .to_block(to_block);

    let mut logs = provider
        .get_logs(&filter)
        .await
        .context("eth_getLogs for Swap failed")?;

    logs.sort_by(|left, right| {
        // сортируем логи
        (left.block_number, left.transaction_index, left.log_index).cmp(&(
            right.block_number,
            right.transaction_index,
            right.log_index,
        )) // функция сравнения
        // возвращает Ordering::Less, Ordering::Equal, Ordering::Greater
    });

    let mut swap_logs = Vec::with_capacity(logs.len());

    for log in logs {
        let block_number = log.block_number.context("Swap log has no block number")?;

        let transaction_index = log
            .transaction_index
            .context("Swap log has no transaction index")?;

        let transaction_hash = log
            .transaction_hash
            .context("Swap log has no transaction hash")?;

        let log_index = log.log_index.context("Swap log has no log index")?;

        let decoded = log
            .log_decode_validate::<Swap>()
            .context("failed to decode Swap log")?;

        let event = decoded.data();

        if event.id != pool_id {
            bail!("Swap log pool_id does not match requested pool_id");
        }

        swap_logs.push(PoolSwapLog {
            block_number,
            transaction_index,
            transaction_hash,
            log_index,
            amount0: i128::try_from(event.amount0).context("amount0 does not fit into i128")?,
            amount1: i128::try_from(event.amount1).context("amount1 does not fit into i128")?,
        });
    }

    Ok(swap_logs)
}
