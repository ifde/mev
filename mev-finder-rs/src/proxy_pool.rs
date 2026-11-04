// ==============================
// Модуль поиска активного пула по заданной паре токенов, который станет нашим прокси пулом (будем добывать из него транзакции)
// Пока что заглушка. Находит только 1 пул WETH / USDC. 
// https://app.uniswap.org/explore/pools/ethereum/0x5a5c7cab5f55c7ea020e97d4fa6dd5d99270e56ce76afa61d8cbddec0af92060?chart=depth
// ============================== 

use alloy::primitives::{address, b256, Address, B256};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyPool {
    pub pool_id: B256,
    pub currency0: Address,
    pub currency1: Address, 
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
}

#[derive(Clone, Copy, Debug)]
pub struct ProxyPoolQuery {
    pub token_a: Address,
    pub token_b: Address,
} 

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProxySwap { // структура, которая хранит собраную транзакцию 
    pub block_number: u64, 
    pub transaction_index: u64, // номер транзакции в блоке 
    pub transaction_hash: B256,
    pub log_index: u64,
    pub zero_for_one: bool,
    pub amount_in: u128,
}

// Пока что хадркодим единственное возможное значение Прокси пула. 
const WETH_USDC_FIXTURE: ProxyPool = ProxyPool {
    pool_id: b256!(
        "5a5c7cab5f55c7ea020e97d4fa6dd5d99270e56ce76afa61d8cbddec0af92060" // хеш PoolId
    ),
    currency0: address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"), // USDC
    currency1: address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"), // WETH (Wrapped Ether)  
    fee: 200,
    tick_spacing: 4,
    hooks: Address::ZERO,
};

pub fn find(query: ProxyPoolQuery) -> Option<ProxyPool> {
    let (currency0, currency1) = if query.token_a < query.token_b {
        (query.token_a, query.token_b)
    } else {
        (query.token_b, query.token_a)
    };

    if currency0 == WETH_USDC_FIXTURE.currency0
        && currency1 == WETH_USDC_FIXTURE.currency1
    {
        Some(WETH_USDC_FIXTURE)
    } else {
        None
    }
}