// ==============================
// Функция для поиска и валидаци референс-пулов для заданного прокси-пула
// ============================== 

use alloy::primitives::Address;
use anyhow::{bail, Result};

use crate::{
    pool_key::pool_id_from_key, // функция, которая вычисляет poolId по полученным параметрам пула 
    proxy_pool::ProxyPool,
    reference_pool::ReferencePool,
    state_view::{load_pool_state, PoolState}, // возвращает текущую цену, тик и ликвидность пула (хотим, чтобы она была не 0)
    subgraph::{find_reference_pools, FindReferencePoolsRequest}, // ищет референс пулы через SubGraph API  
}; 

pub struct ReferenceSearchRequest {
    pub subgraph_url: String,
    pub api_key: String,
    pub rpc_url: String,
    pub pool_manager: Address,
    pub state_view: Address,
    pub analysis_block: u64,
    pub proxy_pool: ProxyPool,
}

pub struct ValidatedReferencePool {
    pub pool: ReferencePool,
    pub state: PoolState,
}

pub struct ReferenceSearchResult {
    pub discovered_count: usize,
    pub pools: Vec<ValidatedReferencePool>,
}

pub async fn find_validated_reference_pools(
    request: ReferenceSearchRequest,
) -> Result<ReferenceSearchResult> {
    let ReferenceSearchRequest {
        subgraph_url,
        api_key,
        rpc_url,
        pool_manager,
        state_view,
        analysis_block,
        proxy_pool,
    } = request;

    let computed_proxy_pool_id = pool_id_from_key(
        proxy_pool.currency0,
        proxy_pool.currency1,
        proxy_pool.fee,
        proxy_pool.tick_spacing,
        proxy_pool.hooks,
    )?;

    if computed_proxy_pool_id != proxy_pool.pool_id {
        bail!(
            "proxy PoolId does not match computed PoolKey hash: expected={}, actual={}",
            proxy_pool.pool_id,
            computed_proxy_pool_id,
        );
    }

    let candidates = find_reference_pools(FindReferencePoolsRequest {
        subgraph_url,
        api_key,
        pool_manager,
        currency0: proxy_pool.currency0,
        currency1: proxy_pool.currency1,
        excluded_pool_id: proxy_pool.pool_id, // наш прокси пул не может быть референс пулом 
    })
    .await?;

    let discovered_count = candidates.len();
    let mut pools = Vec::new();

    for pool in candidates {
        let computed_pool_id = pool_id_from_key(
            pool.currency0,
            pool.currency1,
            pool.fee,
            pool.tick_spacing,
            pool.hooks,
        )?;

        if computed_pool_id != pool.pool_id {
            bail!(
                "subgraph PoolId does not match computed PoolKey hash: expected={}, actual={}",
                pool.pool_id,
                computed_pool_id,
            );
        }

        let state = load_pool_state(
            &rpc_url,
            state_view,
            pool.pool_id,
            analysis_block,
        )
        .await?;

        if state.active_liquidity == 0 {
            continue;
        }

        pools.push(ValidatedReferencePool { pool, state });
    }

    if pools.is_empty() {
        bail!("no reference pools have active liquidity at analysis_block");
    }

    Ok(ReferenceSearchResult {
        discovered_count,
        pools,
    })
}