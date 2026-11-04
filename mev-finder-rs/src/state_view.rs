use alloy::{
    eips::BlockId,
    primitives::{Address, B256},
    providers::{Provider, ProviderBuilder},
};
use anyhow::{bail, Context, Result};

use crate::v4::IStateView;

pub struct PoolState {
    pub sqrt_price_x96: String,
    pub tick: i32,
    pub active_liquidity: u128,
}

pub async fn load_pool_state(
    rpc_url: &str,
    state_view_address: Address,
    pool_id: B256,
    analysis_block: u64,
) -> Result<PoolState> {
    let rpc_url = rpc_url
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

    let state_view = IStateView::new(state_view_address, provider);

    let slot0 = state_view
        .getSlot0(pool_id)
        .block(BlockId::number(analysis_block))
        .call()
        .await
        .context("StateView.getSlot0 failed")?;

    let active_liquidity = state_view
        .getLiquidity(pool_id)
        .block(BlockId::number(analysis_block))
        .call()
        .await
        .context("StateView.getLiquidity failed")?;

    Ok(PoolState {
        sqrt_price_x96: slot0.sqrtPriceX96.to_string(),
        tick: i32::try_from(slot0.tick)
            .context("StateView tick does not fit i32")?,
        active_liquidity,
    })
}