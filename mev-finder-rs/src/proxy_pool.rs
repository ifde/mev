// ==============================
// Модуль поиска активного пула по заданной паре токенов, который станет нашим прокси пулом (будем добывать из него транзакции)
// ============================== 

use alloy::primitives::{Address, B256};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    cache,
    gateways::{
        state_view::{load_pool_state, PoolState},
        subgraph::SubgraphClient,
    },
    pool_key::pool_id_from_key,
    pools::{
        discovery::{find_pools_for_tokens, FindPoolsForTokensRequest},
        queries::FIND_POOLS_FOR_TOKENS_BY_LIQUIDITY_ASC,
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

    let cache_path = cache::path_for_request(
        "proxy-pools-v1",
        &(
            request.state_view,
            request.analysis_block,
            currency0,
            currency1,
            request.max_candidates,
        ),
    )?;

    if let Some(mut result) = cache::read_json::<FoundProxyPool>(&cache_path)? {
        result.cache_hit = true;
        return Ok(result);
    }

    let client = SubgraphClient::new(request.subgraph_url, request.api_key);
    let candidates = find_pools_for_tokens(FindPoolsForTokensRequest {
        client: &client,
        query: FIND_POOLS_FOR_TOKENS_BY_LIQUIDITY_ASC,
        currency0,
        currency1,
        hooks: Some(Address::ZERO),
        limit: request.max_candidates,
    })
    .await?;

    let discovered_count = candidates.len();

    for candidate in candidates {
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

        cache::write_json(&cache_path, &result)?;
        return Ok(result);
    }

    bail!("no proxy pool has active liquidity at analysis_block")
}
