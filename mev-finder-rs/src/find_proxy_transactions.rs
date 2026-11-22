// ==============================
// Поиск внешних транзакций, в которых прокси-пул сделал один Swap.
// приоритет у транзакций, которые прозошли ранее в блоке (чтобы быстрее было воспроизводить State)
// ==============================

use std::collections::BTreeMap;

use alloy::primitives::{Address, B256};
use alloy::providers::{Provider, ProviderBuilder};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    cache,
    constants::MAX_LOG_BLOCKS,
    gateways::pool_manager::{PoolSwapLog, load_pool_swap_logs},
    find_proxy_pool::ProxyPool,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyTransaction {
    pub block_number: u64,
    pub transaction_index: u64, // номер транзакции в блоке
    pub transaction_hash: B256,
    // transaction.from внешней Ethereum-транзакции, не sender события PoolManager.Swap.
    pub transaction_sender: Address, // отправитель транзакции 
    pub block_timestamp: u64, 
    pub block_base_fee_per_gas: u128, // стоимость газа в блоке 
    pub log_index: u64,
    pub zero_for_one: bool,
    pub amount_in: u128,
    pub amount0: i128,
    pub amount1: i128,
}

// Данные одного события Swap до загрузки transaction.from и header блока.
// Эти поля запрашиваем только после отбора одной транзакции на блок.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProxySwap {
    block_number: u64,
    transaction_index: u64,
    transaction_hash: B256,
    log_index: u64,
    zero_for_one: bool,
    amount_in: u128,
    amount0: i128,
    amount1: i128,
}

fn swap_input(amount0: i128, amount1: i128) -> Result<(bool, u128)> {
    if amount0 < 0 && amount1 > 0 {
        Ok((true, amount0.unsigned_abs()))
    } else if amount0 > 0 && amount1 < 0 {
        Ok((false, amount1.unsigned_abs()))
    } else {
        bail!("invalid Swap deltas: amount0={amount0}, amount1={amount1}")
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::B256;

    use super::{ProxySwap, select_first_transaction_per_block, swap_input};

    #[test]
    fn v4_negative_delta_is_input() {
        // Реальное событие proxy-swap: WETH -> USDC.
        assert_eq!(
            swap_input(40_060_464, -17_882_257_086_600_793).unwrap(),
            (false, 17_882_257_086_600_793)
        );
        assert_eq!(
            swap_input(-94_649_143, 42_283_485_321_913_167).unwrap(),
            (true, 94_649_143)
        );
        assert!(swap_input(1, 1).is_err());
        assert!(swap_input(0, 0).is_err());
    }

    #[test]
    fn selects_only_first_proxy_transaction_in_each_block() {
        // функция, которая создает транзакцию 
        let transaction = |block_number, transaction_index, hash| ProxySwap {
            block_number,
            transaction_index,
            transaction_hash: B256::with_last_byte(hash),
            log_index: 0,
            zero_for_one: true,
            amount_in: 1,
            amount0: -1,
            amount1: 1,
        };

        let selected = select_first_transaction_per_block(vec![
            transaction(12, 11, 1),
            transaction(11, 7, 2),
            transaction(12, 3, 3),
            transaction(11, 9, 4),
        ]);

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].transaction_index, 7);
        assert_eq!(selected[1].transaction_index, 3);
    }
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
    pub selected_block_count: usize,
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
    let cache_request = (
        request.expected_chain_id,
        request.pool_manager,
        request.proxy_pool.pool_id,
        request.from_block,
        request.to_block,
        request.max_transactions,
    );
    let cached = cache::load_or_compute("proxy-transactions-v6", &cache_request, || {
        load_proxy_transactions_uncached(request)
    })
    .await?;

    let mut result = cached.value;
    result.cache_hit = cached.cache_hit;
    Ok(result)
}

async fn load_proxy_transactions_uncached(
    request: LoadProxyTransactionsRequest,
) -> Result<LoadedProxyTransactions> {
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
    let mut swaps_by_transaction = BTreeMap::<B256, Vec<ProxySwap>>::new();

    for PoolSwapLog {
        block_number,
        transaction_index,
        transaction_hash,
        log_index,
        amount0,
        amount1,
    } in logs
    {
        // Дельты записаны со стороны вызывающего swap.
        // Отрицательная amount0 означает, что пул получил currency0:
        // пользователь сделал currency0 -> currency1, то есть zeroForOne = true.
        let (zero_for_one, amount_in) = swap_input(amount0, amount1)?;
        let transaction = ProxySwap {
            block_number,
            transaction_index,
            transaction_hash,
            log_index,
            zero_for_one,
            amount_in,
            amount0,
            amount1,
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

    // Одно независимое наблюдение на блок: берём первый proxy-swap блока.
    let mut transactions = select_first_transaction_per_block(transactions);
    let selected_block_count = transactions.len();

    // Показывает, насколько далеко в блоке находится худшая из десяти
    // наиболее ранних proxy-транзакций. Меньше значение — быстрее fork по tx hash.
    let top_10_max_transaction_index = transactions
        .iter()
        .take(10)
        .map(|transaction| transaction.transaction_index)
        .max();

    transactions.truncate(request.max_transactions);
    let transactions = load_transaction_context(
        &request.rpc_url,
        request.expected_chain_id,
        transactions,
    )
    .await?;

    let result = LoadedProxyTransactions {
        found_log_count,
        found_transaction_count,
        skipped_multi_swap_transaction_count,
        selected_block_count,
        top_10_max_transaction_index,
        transactions,
        cache_hit: false,
    };

    Ok(result)
}

async fn load_transaction_context(
    rpc_url: &str,
    expected_chain_id: u64,
    swaps: Vec<ProxySwap>,
) -> Result<Vec<ProxyTransaction>> {
    let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
    if provider.get_chain_id().await? != expected_chain_id {
        bail!("expected chain_id={expected_chain_id}");
    }

    let mut transactions = Vec::with_capacity(swaps.len());
    for swap in swaps {
        let transaction = provider
            .get_transaction_by_hash(swap.transaction_hash)
            .await?
            .context("historical transaction not found")?;
        let block = provider
            .get_block_by_number(swap.block_number.into())
            .await?
            .context("historical block not found")?;

        transactions.push(ProxyTransaction {
            block_number: swap.block_number,
            transaction_index: swap.transaction_index,
            transaction_hash: swap.transaction_hash,
            transaction_sender: transaction.inner.signer(),
            block_timestamp: block.header.timestamp,
            block_base_fee_per_gas: u128::from(block.header.base_fee_per_gas.unwrap_or(0)),
            log_index: swap.log_index,
            zero_for_one: swap.zero_for_one,
            amount_in: swap.amount_in,
            amount0: swap.amount0,
            amount1: swap.amount1,
        });
    }

    Ok(transactions)
}

fn select_first_transaction_per_block(mut transactions: Vec<ProxySwap>) -> Vec<ProxySwap> {
    transactions.sort_by_key(|transaction| {
        (
            transaction.block_number,
            transaction.transaction_index,
            transaction.log_index,
        )
    });

    let mut first_by_block = BTreeMap::<u64, ProxySwap>::new();
    for transaction in transactions {
        first_by_block
            .entry(transaction.block_number)
            .or_insert(transaction);
    }
    first_by_block.into_values().collect()
}
