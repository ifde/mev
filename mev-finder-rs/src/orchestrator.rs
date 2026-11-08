// ==============================
// Оркестрирует другие блоки: находит прокси пул, транзакции прокси пула, 
// референс пулы, маршруты, evaluations 
// ============================== 

use alloy::primitives::Address;
use anyhow::Result;
use serde::Serialize;

use crate::{
    constants::{MAX_PROXY_POOL_CANDIDATES, MAX_REFERENCE_POOL_CANDIDATES},
    proxy_pool::{find_proxy_pool, FindProxyPoolRequest, ProxyPool},
    proxy_transactions::{load_proxy_transactions, LoadProxyTransactionsRequest, ProxyTransaction},
    reference_search::{find_validated_reference_pools, ReferenceSearchRequest, ValidatedReferencePool},
    route::{find_two_pool_routes, Route, RouteKind},
};

pub struct BuildEvaluationPlanRequest {
    pub subgraph_url: String,
    pub api_key: String,
    pub rpc_url: String,
    pub expected_chain_id: u64,
    pub pool_manager: Address,
    pub state_view: Address,
    pub analysis_block: u64,
    pub token_a: Address,
    pub token_b: Address,
    pub from_block: u64,
    pub to_block: u64,
    pub max_transactions: usize,
}

#[derive(Serialize)]
pub struct PlannedRoute {
    pub route_id: usize,
    pub route: Route,
}

#[derive(Serialize)] // сетка цен, на которой будем проверять маршрут в Foundry 
pub struct RouteEvaluation {
    pub route_id: usize,
    pub amount_anchor: u128, // величина исходного свопа (понятно, что свопать более нее нет смысла, так как пропадет перевес в цене) 
    pub grid_ppm: Vec<u32>, // доли amount_anchor: 1_000_000 ppm = 100%
}

#[derive(Serialize)]
pub struct TransactionPlan { // все маршруты для конкретной транзакции 
                             // по сути, ее создает функция build_route_evaluations() ниже в этом файле
    pub proxy_transaction: ProxyTransaction,
    pub route_evaluations: Vec<RouteEvaluation>,
}

#[derive(Serialize)] 
pub struct EvaluationPlan { // результат работы оркестратора: прокси пул, референс пулы, маршруты, конкретные evaluations (единицы работы Foundry) 
    pub schema_version: u32, // версия формата вот этой структуры EvaluationPlan
    pub chain_id: u64,
    pub analysis_block: u64,
    pub proxy_pool: ProxyPool,
    pub reference_pools: Vec<ValidatedReferencePool>,
    pub routes: Vec<PlannedRoute>,
    pub transaction_plans: Vec<TransactionPlan>,
}

pub async fn build_evaluation_plan(
    request: BuildEvaluationPlanRequest,
) -> Result<EvaluationPlan> {
    let proxy = find_proxy_pool(FindProxyPoolRequest {
        subgraph_url: request.subgraph_url.clone(),
        api_key: request.api_key.clone(),
        rpc_url: request.rpc_url.clone(),
        state_view: request.state_view,
        analysis_block: request.analysis_block,
        token_a: request.token_a,
        token_b: request.token_b,
        max_candidates: MAX_PROXY_POOL_CANDIDATES,
    })
    .await?;

    let references = find_validated_reference_pools(ReferenceSearchRequest {
        subgraph_url: request.subgraph_url.clone(),
        api_key: request.api_key.clone(),
        rpc_url: request.rpc_url.clone(),
        state_view: request.state_view,
        analysis_block: request.analysis_block,
        proxy_pool: proxy.pool,
        max_reference_pools: MAX_REFERENCE_POOL_CANDIDATES,
    })
    .await?;

    let reference_pools = references
        .pools
        .iter()
        .map(|candidate| candidate.pool.clone())
        .collect::<Vec<_>>();
    let routes = find_two_pool_routes(&proxy.pool, &reference_pools)?;

    let transactions = load_proxy_transactions(LoadProxyTransactionsRequest {
        rpc_url: request.rpc_url,
        expected_chain_id: request.expected_chain_id,
        pool_manager: request.pool_manager,
        proxy_pool: proxy.pool,
        from_block: request.from_block,
        to_block: request.to_block,
        max_transactions: request.max_transactions,
    })
    .await?;

    let planned_routes = routes
        .into_iter()
        .enumerate()
        .map(|(route_id, route)| PlannedRoute { route_id, route })
        .collect::<Vec<_>>();

    let transaction_plans = transactions
        .transactions
        .iter()
        .map(|transaction| TransactionPlan {
            proxy_transaction: *transaction, // dereference transaction to make it an owned value 
            route_evaluations: build_route_evaluations(
                &proxy.pool,
                &planned_routes,
                transaction,
            ),
        })
        .collect::<Vec<_>>();

    Ok(EvaluationPlan {
        schema_version: 1,
        chain_id: request.expected_chain_id,
        analysis_block: request.analysis_block,
        proxy_pool: proxy.pool,
        reference_pools: references.pools,
        routes: planned_routes,
        transaction_plans,
    })
}

// принимает конкретную транзакцию и все возможные маршруты 
// оставляет только потенциально прибыльные пары (transaction, route) для проверки 
fn build_route_evaluations(
    proxy_pool: &ProxyPool,
    routes: &[PlannedRoute],
    transaction: &ProxyTransaction,
) -> Vec<RouteEvaluation> {
    let (user_input, user_output) = if transaction.zero_for_one {
        (proxy_pool.currency0, proxy_pool.currency1)
    } else {
        (proxy_pool.currency1, proxy_pool.currency0)
    };
    let mut evaluations = Vec::new();

    for planned_route in routes {
        let is_direct = planned_route.route.kind == RouteKind::Direct
            && planned_route.route.start_currency == user_output;
        let is_reverse = planned_route.route.kind == RouteKind::Reverse
            && planned_route.route.start_currency == user_input;

        if !is_direct && !is_reverse {
            continue;
        }

        let amount_anchor = if planned_route.route.start_currency == proxy_pool.currency0 {
            transaction.amount0.unsigned_abs()
        } else {
            transaction.amount1.unsigned_abs()
        };

        evaluations.push(RouteEvaluation {
            route_id: planned_route.route_id,
            amount_anchor,
            grid_ppm: vec![10, 50, 100, 500, 1_000],
        });
    }

    evaluations
}
