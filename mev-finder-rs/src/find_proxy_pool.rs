// ==============================
// Модуль поиска активного пула по заданной паре токенов, который станет нашим прокси пулом (будем добывать из него транзакции)
// ==============================

use alloy::primitives::{Address, B256};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    cache,
    gateways::{
        state_view::{PoolState, load_pool_state},
        subgraph::SubgraphClient,
    },
    pools::{
        discovery::{FindPoolsForTokensRequest, find_pools_for_tokens},
        key::pool_id_from_key,
        queries::FIND_POOLS_FOR_TOKENS_BY_LIQUIDITY_DESC,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyPool {
    pub pool_id: B256,
    pub currency0: Address,
    pub currency1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
}

pub struct FindProxyPoolRequest {
    pub subgraph_url: String,
    pub api_key: String,
    pub rpc_url: String,
    pub state_view: Address,
    pub analysis_block: u64,
    pub token_a: Address,
    pub token_b: Address,
    /// Если задан, не выбираем максимум: используем этот proxy PoolId.
    pub proxy_pool_id: Option<B256>,
    pub max_candidates: usize,
}

#[derive(Serialize, Deserialize)]
pub struct FoundProxyPool {
    pub pool: ProxyPool,
    pub state: PoolState,
    pub discovered_count: usize,
    #[serde(skip)]
    pub cache_hit: bool,
}

pub async fn find_proxy_pool(request: FindProxyPoolRequest) -> Result<FoundProxyPool> {
    let (currency0, currency1) = if request.token_a < request.token_b {
        (request.token_a, request.token_b)
    } else {
        (request.token_b, request.token_a)
    };

    let cache_request = (
        request.state_view,
        request.analysis_block,
        currency0,
        currency1,
        request.proxy_pool_id,
        request.max_candidates,
    );
    let cached = cache::load_or_compute("proxy-pools-v2", &cache_request, || {
        find_proxy_pool_uncached(request, currency0, currency1)
    })
    .await?;

    let mut result = cached.value;
    result.cache_hit = cached.cache_hit;
    Ok(result)
}

async fn find_proxy_pool_uncached(
    request: FindProxyPoolRequest,
    currency0: Address,
    currency1: Address,
) -> Result<FoundProxyPool> {
    let client = SubgraphClient::new(request.subgraph_url, request.api_key);
    let candidates = find_pools_for_tokens(FindPoolsForTokensRequest {
        client: &client,
        query: FIND_POOLS_FOR_TOKENS_BY_LIQUIDITY_DESC,
        currency0,
        currency1,
        hooks: Some(Address::ZERO),
        limit: request.max_candidates,
    })
    .await?;

    let discovered_count = candidates.len();

    let mut selected: Option<FoundProxyPool> = None;
    for candidate in candidates {
        if let Some(proxy_pool_id) = request.proxy_pool_id {
            if candidate.pool_id != proxy_pool_id {
                continue;
            }
        }

        let computed_pool_id = pool_id_from_key(
            candidate.currency0,
            candidate.currency1,
            candidate.fee,
            candidate.tick_spacing,
            candidate.hooks,
        )?;

        if computed_pool_id != candidate.pool_id {
            bail!("subgraph PoolId does not match computed PoolKey hash");
        }

        let state = load_pool_state(
            &request.rpc_url,
            request.state_view,
            candidate.pool_id,
            request.analysis_block,
        )
        .await?;

        if state.active_liquidity == 0 {
            continue;
        }

        let active_liquidity = state.active_liquidity;
        let result = FoundProxyPool {
            pool: ProxyPool {
                pool_id: candidate.pool_id,
                currency0: candidate.currency0,
                currency1: candidate.currency1,
                fee: candidate.fee,
                tick_spacing: candidate.tick_spacing,
                hooks: candidate.hooks,
            },
            state,
            discovered_count,
            cache_hit: false,
        };

        if request.proxy_pool_id.is_some() {
            return Ok(result);
        }

        if selected.as_ref().is_none_or(|current| {
            active_liquidity > current.state.active_liquidity
                || (active_liquidity == current.state.active_liquidity
                    && result.pool.pool_id < current.pool.pool_id)
        }) {
            selected = Some(result);
        }
    }

    selected.context("no proxy pool has active liquidity at analysis_block")
}
