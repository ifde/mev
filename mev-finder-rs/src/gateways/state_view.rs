// ==============================
// Получить StateView пула на analysis_block. А именно, цену, тик и активную ликвидность
// ==============================

use std::time::Duration;

use alloy::{
    eips::BlockId,
    primitives::{Address, B256},
    providers::{Provider, ProviderBuilder},
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::v4::IStateView;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    const RETRY_DELAYS: [Duration; 3] = [
        Duration::from_secs(1),
        Duration::from_secs(2),
        Duration::from_secs(4),
    ];

    for delay in RETRY_DELAYS {
        match load_pool_state_once(rpc_url, state_view_address, pool_id, analysis_block).await {
            Ok(state) => return Ok(state),
            Err(error) if is_rate_limited(&error) => {
                eprintln!(
                    "StateView rate limited for {pool_id}; retrying in {}s",
                    delay.as_secs()
                );
                tokio::time::sleep(delay).await;
            }
            Err(error) => return Err(error),
        }
    }

    load_pool_state_once(rpc_url, state_view_address, pool_id, analysis_block).await
}

async fn load_pool_state_once(
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
        tick: i32::try_from(slot0.tick).context("StateView tick does not fit i32")?,
        active_liquidity,
    })
}

fn is_rate_limited(error: &anyhow::Error) -> bool {
    let message = format!("{error:#}");
    message.contains("HTTP 429") || message.contains("Too Many Requests")
}
