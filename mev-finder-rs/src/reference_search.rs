// ==============================
// Функция для поиска и валидаци референс-пулов для заданного прокси-пула
// возвращает пару (pool, state)  
/* 
"pool": {
    "pool_id": "0x4f88f7c99022eace4740c6898f59ce6a2e798a1e64ce54589720b7153eb224a7",
    "currency0": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
    "currency1": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
    "fee": 500,
    "tick_spacing": 10,
    "hooks": "0x0000000000000000000000000000000000000000",
    "active_liquidity": 5077704143538248,
    "total_value_locked_usd": "88943.36681261352766757287127170556",
    "tx_count": 441125
    },
    "state": {
    "sqrt_price_x96": "1663201486119228121127357328770254",
    "tick": 199048,
    "active_liquidity": 5006708802601625
    }
*/ 
// ============================== 

use alloy::primitives::Address;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::{
    cache,
    gateways::{
        state_view::{load_pool_state, PoolState},
        subgraph::SubgraphClient,
    },
    pool_key::pool_id_from_key, // функция, которая вычисляет poolId по полученным параметрам пула 
    pools::{
        discovery::{find_pools_for_tokens, FindPoolsForTokensRequest},
        queries::FIND_ACTIVE_POOLS_FOR_TOKENS,
    },
    proxy_pool::ProxyPool,
    reference_pool::ReferencePool,
}; 

pub struct ReferenceSearchRequest {
    pub subgraph_url: String,
    pub api_key: String,
    pub rpc_url: String,
    pub state_view: Address,
    pub analysis_block: u64,
    pub proxy_pool: ProxyPool,
    pub max_reference_pools: usize,
}

#[derive(Serialize, Deserialize)]
pub struct ValidatedReferencePool {
    pub pool: ReferencePool,
    pub state: PoolState,
}

#[derive(Serialize, Deserialize)]
pub struct ReferenceSearchResult {
    pub discovered_count: usize,
    pub pools: Vec<ValidatedReferencePool>,
    #[serde(skip)]
    pub cache_hit: bool,
}

pub async fn find_validated_reference_pools(
    request: ReferenceSearchRequest,
) -> Result<ReferenceSearchResult> {
    let ReferenceSearchRequest {
        subgraph_url,
        api_key,
        rpc_url,
        state_view,
        analysis_block,
        proxy_pool,
        max_reference_pools,
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

    let cache_path = cache::path_for_request(
        "reference-pools-v2",
        &(
            state_view,
            analysis_block,
            proxy_pool,
            max_reference_pools,
        ),
    )?;

    if let Some(mut result) = cache::read_json::<ReferenceSearchResult>(&cache_path)? {
        result.cache_hit = true;
        return Ok(result);
    }

    // Клиент знает только, как отправить GraphQL-запрос.
    // Кверя и правила отбора пула принадлежат модулю pools.
    let client = SubgraphClient::new(subgraph_url, api_key);
    let candidates = find_pools_for_tokens(FindPoolsForTokensRequest {
        client: &client,
        query: FIND_ACTIVE_POOLS_FOR_TOKENS,
        currency0: proxy_pool.currency0,
        currency1: proxy_pool.currency1,
        hooks: None,
        limit: max_reference_pools,
    })
    .await?;

    let discovered_count = candidates.len();
    let mut reference_pools = Vec::new();

    for candidate in candidates {
        if candidate.pool_id == proxy_pool.pool_id {
            continue; // наш прокси пул не может быть референс пулом
        }

        let pool = ReferencePool {
            pool_id: candidate.pool_id,
            currency0: candidate.currency0,
            currency1: candidate.currency1,
            fee: candidate.fee,
            tick_spacing: candidate.tick_spacing,
            hooks: candidate.hooks,
            active_liquidity: candidate.subgraph_liquidity,
            total_value_locked_usd: candidate.total_value_locked_usd,
            tx_count: candidate.tx_count,
        };

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

        reference_pools.push(pool);
    }

    // Состояние каждого пула не зависит от остальных, поэтому запрашиваем его параллельно.
    let mut state_tasks = JoinSet::new();

    for (rank, pool) in reference_pools.into_iter().enumerate() {
        let rpc_url = rpc_url.clone();

        state_tasks.spawn(async move {
            let state = load_pool_state(&rpc_url, state_view, pool.pool_id, analysis_block).await?;
            Ok::<_, anyhow::Error>((rank, pool, state))
        });
    }

    let mut ranked_pools = Vec::new();

    while let Some(task) = state_tasks.join_next().await {
        let (rank, pool, state) = task
            .context("reference pool state task failed")??;

        if state.active_liquidity != 0 {
            ranked_pools.push((rank, ValidatedReferencePool { pool, state }));
        }
    }

    ranked_pools.sort_by_key(|(rank, _)| *rank);
    let pools = ranked_pools
        .into_iter()
        .map(|(_, pool)| pool)
        .collect::<Vec<_>>();

    if pools.is_empty() {
        bail!("no reference pools have active liquidity at analysis_block");
    }

    let result = ReferenceSearchResult {
        discovered_count,
        pools,
        cache_hit: false,
    };

    cache::write_json(&cache_path, &result)?;

    Ok(result)
}
