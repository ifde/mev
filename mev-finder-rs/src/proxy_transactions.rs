// ==============================
// Поиск внешних транзакций, в которых прокси-пул сделал один Swap.
// приоритет у транзакций, которые прозошли ранее в блоке (чтобы быстрее было воспроизводить State) 
// ==============================

use std::collections::BTreeMap;

use alloy::primitives::{Address, B256};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    cache,
    constants::MAX_LOG_BLOCKS,
    gateways::pool_manager::{load_pool_swap_logs, PoolSwapLog},
    proxy_pool::ProxyPool,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyTransaction {
    pub block_number: u64,
    pub transaction_index: u64, // номер транзакции в блоке
    pub transaction_hash: B256,
    pub log_index: u64,
    pub zero_for_one: bool,
    pub amount_in: u128,
    pub amount0: i128,
    pub amount1: i128,
}

pub struct LoadProxyTransactionsRequest {
    pub rpc_url: String,
    pub expected_chain_id: u64,
    pub pool_manager: Address,
    pub proxy_pool: ProxyPool,
    pub from_block: u64,
    pub to_block: u64,
    pub max_transactions: usize,
}

#[derive(Serialize, Deserialize)]
pub struct LoadedProxyTransactions {
    pub found_log_count: usize,
    pub found_transaction_count: usize,
    pub skipped_multi_swap_transaction_count: usize,
    pub top_10_max_transaction_index: Option<u64>,
    pub transactions: Vec<ProxyTransaction>,
    #[serde(skip)]
    pub cache_hit: bool,
}

pub async fn load_proxy_transactions(
    request: LoadProxyTransactionsRequest,
) -> Result<LoadedProxyTransactions> {
    if request.from_block > request.to_block {
        bail!("from_block must not be greater than to_block");
    }

    // rpc_url не записываем: в нём может находиться API-ключ провайдера.
    let cache_path = cache::path_for_request(
        "proxy-transactions-v3",
        &(
            request.expected_chain_id,
            request.pool_manager,
            request.proxy_pool.pool_id,
            request.from_block,
            request.to_block,
            request.max_transactions,
        ),
    )?;

    // если уже искали транзакции с теми же параметрами, просто возвращаем результат
    if let Some(mut result) = cache::read_json::<LoadedProxyTransactions>(&cache_path)? {
        result.cache_hit = true;
        return Ok(result);
    }

    // RPC провайдер ограничивает eth_getLogs диапазоном в 10 000 блоков.
    // Большой пользовательский диапазон читаем последовательно одинаковыми пакетами.
    let mut logs = Vec::new();
    let mut batch_from = request.from_block;

    loop {
        let batch_to = batch_from
            .saturating_add(MAX_LOG_BLOCKS - 1)
            .min(request.to_block);

        logs.extend(
            load_pool_swap_logs(
                &request.rpc_url,
                request.expected_chain_id,
                request.pool_manager,
                request.proxy_pool.pool_id,
                batch_from,
                batch_to,
            )
            .await?,
        );

        if batch_to == request.to_block {
            break;
        }

        batch_from = batch_to + 1;
    }

    let found_log_count = logs.len();
    // маппинг хеша транзакции в саму транзакцию 
    let mut swaps_by_transaction = BTreeMap::<B256, Vec<ProxyTransaction>>::new();

    for PoolSwapLog {
        block_number,
        transaction_index,
        transaction_hash,
        log_index,
        amount0,
        amount1,
    } in logs {

        // Дельты записаны со стороны PoolManager.
        // Положительная amount0 означает, что пул получил currency0:
        // пользователь сделал currency0 -> currency1, то есть zeroForOne = true.
        let transaction = if amount0 > 0 && amount1 < 0 {
            ProxyTransaction {
                block_number,
                transaction_index,
                transaction_hash,
                log_index,
                zero_for_one: true,
                amount_in: amount0 as u128,
                amount0,
                amount1,
            }
        } else if amount0 < 0 && amount1 > 0 {
            ProxyTransaction {
                block_number,
                transaction_index,
                transaction_hash,
                log_index,
                zero_for_one: false,
                amount_in: amount1 as u128,
                amount0,
                amount1,
            }
        } else {
            bail!("invalid Swap deltas: amount0={amount0}, amount1={amount1}");
        };

        swaps_by_transaction
            .entry(transaction_hash)
            .or_default()
            .push(transaction);
    }

    let found_transaction_count = swaps_by_transaction.len();
    let mut skipped_multi_swap_transaction_count = 0;
    let mut transactions = Vec::with_capacity(found_transaction_count);

    for swaps in swaps_by_transaction.into_values() {
        if swaps.len() != 1 {
            skipped_multi_swap_transaction_count += 1;
            continue;
        }

        transactions.push(swaps[0]);
    }

    // Для Foundry сначала берём транзакции с наименьшим индексом в блоке 
    // так как состояние непосредственно перед ними быстрее восстановить по transaction hash.
    transactions.sort_by_key(|transaction| {
        (
            transaction.transaction_index,
            transaction.block_number,
            transaction.log_index,
        )
    });

    // Показывает, насколько далеко в блоке находится худшая из десяти
    // наиболее ранних proxy-транзакций. Меньше значение — быстрее fork по tx hash.
    let top_10_max_transaction_index = transactions
        .iter()
        .take(10)
        .map(|transaction| transaction.transaction_index)
        .max();

    transactions.truncate(request.max_transactions);

    let result = LoadedProxyTransactions {
        found_log_count,
        found_transaction_count,
        skipped_multi_swap_transaction_count,
        top_10_max_transaction_index,
        transactions,
        cache_hit: false,
    };

    cache::write_json(&cache_path, &result)?;

    Ok(result)
}
