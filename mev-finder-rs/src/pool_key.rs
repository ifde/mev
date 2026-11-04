// ==============================
// По параметрам пула создает PoolKey, хеширует и получает PoolId (повторяем логику из Solidity)
// ============================== 

use alloy::{
    primitives::{keccak256, B256, aliases::{I24, U24}, Address},
    sol_types::SolValue,
}; 

use anyhow::{Context, Result}; 

use crate::{
    v4::PoolKey,
};

pub fn pool_id_from_key(
    currency0: Address,
    currency1: Address,
    fee: u32,
    tick_spacing: i32,
    hooks: Address,
) -> Result<B256> {
    let tick_spacing = I24::try_from(tick_spacing)
        .context("tick_spacing does not fit Solidity int24")?;

    let key = PoolKey {
        currency0,
        currency1,
        fee: U24::from(fee),
        tickSpacing: tick_spacing,
        hooks,
    };

    Ok(keccak256(key.abi_encode()))
}