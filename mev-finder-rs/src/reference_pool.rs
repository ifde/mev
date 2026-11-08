// ==============================
// структура для хранения референс пула 
// ============================== 

use alloy::primitives::{Address, B256};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferencePool {
    pub pool_id: B256,
    pub currency0: Address,
    pub currency1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
    pub active_liquidity: u128,
    pub total_value_locked_usd: String,
    pub tx_count: u64,
}
