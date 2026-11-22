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

use alloy::primitives::{Address, B256};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    cache,
    gateways::{
        state_view::{PoolState, load_pool_state},
        subgraph::SubgraphClient,
    },
    pools::{
        discovery::{FindPoolsForTokensRequest, find_pools_for_tokens},
        key::pool_id_from_key, // функция, которая вычисляет poolId по полученным параметрам пула
        queries::FIND_ACTIVE_POOLS_FOR_TOKENS,
    },
    find_proxy_pool::ProxyPool,
};

// Данные reference-пула из Subgraph. Историческое состояние хранится отдельно
// в ValidatedReferencePool.state.
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
    #[serde(skip)] // это поле не записывается и не читается в JSON. То есть, не используется в кеше. 
    pub cache_hit: bool,
}

// функция нуждается в доработке: возможность добавить искать пулы с другими парами токенов (для маршрутов длины 3). 
pub async fn find_validated_reference_pools(
    request: ReferenceSearchRequest,
) -> Result<ReferenceSearchResult> {
    // доп. проверка, что переданные параметры прокси пула соответствуют переданному PoolId 
    let computed_proxy_pool_id = pool_id_from_key(
        request.proxy_pool.currency0,
        request.proxy_pool.currency1,
        request.proxy_pool.fee,
        request.proxy_pool.tick_spacing,
        request.proxy_pool.hooks,
    )?;

    if computed_proxy_pool_id != request.proxy_pool.pool_id {
        bail!(
            "proxy PoolId does not match computed PoolKey hash: expected={}, actual={}",
            request.proxy_pool.pool_id,
            computed_proxy_pool_id,
        );
    }

    let cache_request = (
        request.state_view,
        request.analysis_block,
        request.proxy_pool,
        request.max_reference_pools,
    );
    let cached = cache::load_or_compute("reference-pools-v2", &cache_request, || {
        find_validated_reference_pools_uncached(request)
    })
    .await?;

    let mut result = cached.value;
    result.cache_hit = cached.cache_hit;
    Ok(result)
}

async fn find_validated_reference_pools_uncached(
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

    // Клиент знает только, как отправить GraphQL-запрос.
    // Кверя и правила отбора пула принадлежат модулю pools.
    let client = SubgraphClient::new(subgraph_url, api_key);
    let candidates = find_pools_for_tokens(FindPoolsForTokensRequest {
        client: &client,
        query: FIND_ACTIVE_POOLS_FOR_TOKENS, // этот запрос тоже фильрует по убыванию ликвидности 
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

    let mut ranked_pools = Vec::new();

    // Запрашиваем последовательно активный стейт пулов 
    for (rank, pool) in reference_pools.into_iter().enumerate() {
        let state = load_pool_state(&rpc_url, state_view, pool.pool_id, analysis_block).await?;

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

    Ok(result)
}
