// ==============================
// Типы PoolCandidate 
// ============================== 

use alloy::primitives::{Address, B256};
use serde::{Deserialize, Serialize};

// Пул, найденный через Subgraph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolCandidate {
    pub pool_id: B256,
    pub currency0: Address,
    pub currency1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,

    // Значение из Subgraph на момент его индексирования.
    pub subgraph_liquidity: u128,

    pub total_value_locked_usd: String,
    pub tx_count: u64,
}