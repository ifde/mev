// ==============================
// Поиск Uniswap v4-пулов через Subgraph API.
// Вход: Кверя для SubGraph API с параметрами currency0, currency1, limit и, при необходимости, hooks.
// ==============================

use alloy::primitives::{Address, B256};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::json;

use crate::{gateways::subgraph::SubgraphClient, pools::types::PoolCandidate};

// Вход поиска пулов для двух заданных токенов.
// Кверю query передаёт вызывающий модуль, чтобы он явно выбирал сортировку.
pub struct FindPoolsForTokensRequest<'a> {
    pub client: &'a SubgraphClient,
    pub query: &'a str,
    pub currency0: Address,
    pub currency1: Address,
    // None означает, что адрес hook не ограничиваем.
    pub hooks: Option<Address>,
    pub limit: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoolsData {
    pools: Vec<GraphQlPool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphQlPool {
    id: String,
    token0: GraphQlToken,
    token1: GraphQlToken,
    fee: u32,
    tick_spacing: i32,
    hooks: String,
    liquidity: String,

    #[serde(rename = "totalValueLockedUSD")]
    // переименовываем поле, чтобы оно правильно считалось через serde
    // то есть в SubGraph оно называется totalValueLockedUSD
    total_value_locked_usd: String,

    tx_count: u64,
}

#[derive(Deserialize)]
struct GraphQlToken {
    id: String,
}

pub async fn find_pools_for_tokens(
    request: FindPoolsForTokensRequest<'_>,
) -> Result<Vec<PoolCandidate>> {
    let variables = json!({
        "token0": request.currency0.to_string().to_lowercase(),
        "token1": request.currency1.to_string().to_lowercase(),
        "limit": request.limit,
    });

    let mut variables = variables;
    if let Some(hooks) = request.hooks {
        variables["hooks"] = json!(hooks.to_string().to_lowercase());
    }

    let data: PoolsData = request.client.query(request.query, variables).await?;

    let mut candidates = Vec::with_capacity(data.pools.len());

    for raw_pool in data.pools {
        let pool_id = raw_pool
            .id
            .parse::<B256>()
            .context("subgraph pool id is not bytes32")?;

        let currency0 = raw_pool
            .token0
            .id
            .parse::<Address>()
            .context("subgraph token0 is not an address")?;

        let currency1 = raw_pool
            .token1
            .id
            .parse::<Address>()
            .context("subgraph token1 is not an address")?;

        let hooks = raw_pool
            .hooks
            .parse::<Address>()
            .context("subgraph hooks is not an address")?;

        if currency0 != request.currency0 || currency1 != request.currency1 {
            bail!("subgraph returned a pool that does not match the requested tokens");
        }

        if let Some(expected_hooks) = request.hooks {
            if hooks != expected_hooks {
                bail!("subgraph returned a pool that does not match the requested hooks");
            }
        }

        // добавляем найденный пул в массив.
        // liquidity здесь только сохраненное значение Subgraph.
        // Позже StateView проверит фактическую ликвидность на analysis_block.
        candidates.push(PoolCandidate {
            pool_id,
            currency0,
            currency1,
            fee: raw_pool.fee,
            tick_spacing: raw_pool.tick_spacing,
            hooks,
            subgraph_liquidity: raw_pool
                .liquidity
                .parse::<u128>()
                .context("subgraph liquidity does not fit u128")?,
            total_value_locked_usd: raw_pool.total_value_locked_usd,
            tx_count: raw_pool.tx_count,
        });
    }

    Ok(candidates)
}
