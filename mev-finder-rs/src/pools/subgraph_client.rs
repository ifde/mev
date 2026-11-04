// ==============================
// Поиск активных пулов через Subgraph API
// Отправляет GraphQL-запрос и декодирует ответ.
// ==============================

use alloy::primitives::{Address, B256};
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::pools::types::PoolCandidate;

// Для proxy-пулов ищем от меньшей ликвидности к большей.
const FIND_POOLS_BY_LIQUIDITY_ASC_QUERY: &str = r#"
query FindHooklessPairPools($token0: String!, $token1: String!, $limit: Int!) {
poolManagers(first: 5) {
    id
}

pools(
    first: $limit
    where: {
    token0: $token0
    token1: $token1
    hooks: "0x0000000000000000000000000000000000000000"
    }
    orderBy: liquidity
    orderDirection: asc
) {
    id
    token0 { id }
    token1 { id }
    fee
    tickSpacing
    hooks
    liquidity
    totalValueLockedUSD
    txCount
}
}
"#;

// Для reference-пулов ищем от большего TVL к меньшему.
const FIND_POOLS_BY_TVL_DESC_QUERY: &str = r#"
query FindHooklessPairPools($token0: String!, $token1: String!, $limit: Int!) {
poolManagers(first: 5) {
    id
}

pools(
    first: $limit
    where: {
    token0: $token0
    token1: $token1
    hooks: "0x0000000000000000000000000000000000000000"
    }
    orderBy: totalValueLockedUSD
    orderDirection: desc
) {
    id
    token0 { id }
    token1 { id }
    fee
    tickSpacing
    hooks
    liquidity
    totalValueLockedUSD
    txCount
}
}
"#;

// Как Subgraph должен упорядочить найденные пулы.
pub enum PoolOrder {
    LiquidityAscending,
    TotalValueLockedDescending,
}

// конкретный вход клиента Subgraph
pub struct FindPairPoolsRequest {
    pub subgraph_url: String,
    pub api_key: String,

    // просто проверим, что pool_manager будет именно с нужной сети
    // условно, Ethereum Mainnet
    pub pool_manager: Address,

    pub currency0: Address,
    pub currency1: Address,
    pub limit: usize,
    pub order: PoolOrder,
}

// конкретный GraphQL-запрос
#[derive(Serialize)]
struct GraphQlRequest<'a> {
    query: &'a str,
    variables: Value, // передаем параметры через JSON
}

// ответ
// везде в ответах добавляем трейт Deserialize
#[derive(Deserialize)]
struct GraphQlResponse {
    data: Option<GraphQlData>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphQlData {
    pool_managers: Vec<GraphQlPoolManager>,
    pools: Vec<GraphQlPool>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Deserialize)]
struct GraphQlPoolManager {
    id: String,
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
    total_value_locked_usd: String,

    tx_count: u64,
}

#[derive(Deserialize)]
struct GraphQlToken {
    id: String,
}

// запрос на поиск hookless v4-пулов одной пары через Subgraph API
pub async fn find_hookless_pair_pools(
    request: FindPairPoolsRequest,
) -> Result<Vec<PoolCandidate>> {
    let query = match request.order {
        PairPoolOrder::LiquidityAscending => FIND_POOLS_BY_LIQUIDITY_ASC_QUERY,
        PairPoolOrder::TotalValueLockedDescending => FIND_POOLS_BY_TVL_DESC_QUERY,
    };

    let variables = json!({
        "token0": request.currency0.to_string().to_lowercase(),
        "token1": request.currency1.to_string().to_lowercase(),
        "limit": request.limit,
    });

    // делаем API запрос к SubGraph API
    let response = Client::new()
        .post(&request.subgraph_url)
        .bearer_auth(&request.api_key)
        .json(&GraphQlRequest {
            query,
            variables,
        })
        .send()
        .await
        .context("subgraph HTTP request failed")?
        .error_for_status()
        .context("subgraph returned non-success HTTP status")?
        // преобразуем JSON в типы Rust, включая все вложенные поля
        .json::<GraphQlResponse>()
        .await
        .context("failed to decode subgraph response")?;

    if let Some(errors) = response.errors {
        let messages = errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join(", ");

        bail!("subgraph returned errors: {messages}");
    }

    let data = response.data.context("subgraph response has no data")?;

    // проверка, что pool_manager из API ответа совпадает с тем,
    // который мы ожидаем для Ethereum Mainnet
    let has_expected_pool_manager = data.pool_managers.iter().any(|manager| {
        manager.id.parse::<Address>().ok() == Some(request.pool_manager)
    });

    if !has_expected_pool_manager {
        bail!("subgraph does not contain the configured Mainnet PoolManager");
    }

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

        if currency0 != request.currency0
            || currency1 != request.currency1
            || hooks != Address::ZERO
        {
            bail!("subgraph returned a pool that does not match the requested pair");
        }

        // добавляем найденный пул в массив
        // liquidity здесь только значение Subgraph.
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