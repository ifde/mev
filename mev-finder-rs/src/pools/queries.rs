// ==============================
// GraphQL-запросы, относящиеся только к Uniswap v4-пулам.
// Вызывающий модуль выбирает, какой запрос передать discovery.
// ==============================

// Для proxy-пулов ищем от большей ликвидности к меньшей.
pub const FIND_POOLS_FOR_TOKENS_BY_LIQUIDITY_DESC: &str = r#"
query FindPoolsForTokens($token0: String!, $token1: String!, $hooks: String!, $limit: Int!) {
pools(
    first: $limit
    where: {
    token0: $token0
    token1: $token1
    hooks: $hooks
    liquidity_gt: "0"
    }
    orderBy: liquidity
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

// Для reference-пулов не ограничиваем адрес hook.
// liquidity_gt — быстрый фильтр по текущему состоянию индексатора.
// StateView ниже всё равно проверяет ликвидность на analysis_block.
pub const FIND_ACTIVE_POOLS_FOR_TOKENS: &str = r#"
query FindPoolsForTokens($token0: String!, $token1: String!, $limit: Int!) {
pools(
    first: $limit
    where: {
    token0: $token0
    token1: $token1
    liquidity_gt: "0"
    }
    orderBy: liquidity
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
