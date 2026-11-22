use alloy::eips::BlockId;
use anyhow::{Context, Result};

use crate::{
    cli::QuoteTwoPoolCycleArgs,
    constants::{MAX_PROXY_POOL_CANDIDATES, MAX_REFERENCE_POOL_CANDIDATES},
    gateways::quoter::quote_exact_input,
    find_proxy_pool::{FindProxyPoolRequest, find_proxy_pool},
    find_reference_pools::{ReferenceSearchRequest, find_validated_reference_pools},
    route::{RouteKind, find_two_pool_routes, quote_exact_params},
};

pub async fn run(args: QuoteTwoPoolCycleArgs) -> Result<()> {
    let search = args.reference_search;

    // находим прокси пул
    let proxy = find_proxy_pool(FindProxyPoolRequest {
        subgraph_url: search.subgraph_url.clone(),
        api_key: search.api_key.clone(),
        rpc_url: search.rpc_url.clone(),
        state_view: search.state_view,
        analysis_block: search.analysis_block,
        token_a: search.token_a,
        token_b: search.token_b,
        proxy_pool_id: None,
        max_candidates: MAX_PROXY_POOL_CANDIDATES,
    })
    .await?;
    let proxy_pool = proxy.pool;

    // находим референс пулы
    let result = find_validated_reference_pools(ReferenceSearchRequest {
        subgraph_url: search.subgraph_url,
        api_key: search.api_key,
        rpc_url: search.rpc_url.clone(), // клонируем, так как далее будем еще раз использовать это поле
        state_view: search.state_view,
        analysis_block: search.analysis_block,
        proxy_pool,
        max_reference_pools: MAX_REFERENCE_POOL_CANDIDATES,
    })
    .await?;

    // Достаем только первый референс пул (потом добавим больше)
    let reference = result
        .pools
        .first()
        .context("no validated reference pool was found")?;

    let routes = find_two_pool_routes(&proxy_pool, &[reference.pool.clone()])?;
    let route = routes
        .iter()
        .find(|route| {
            matches!(route.kind, RouteKind::Direct) && route.start_currency == args.start_currency
        })
        .context("start_currency is not in the proxy pool")?;

    // Создаем параметры маршрута
    let params = quote_exact_params(route, args.amount_in)?;

    // вызываем V4Quoter для проверки маршрута
    let (quote_rpc_url, quote_block) = match args.anvil_rpc_url.as_deref() {
        Some(url) => (url, BlockId::latest()),
        None => (
            search.rpc_url.as_str(),
            BlockId::number(search.analysis_block),
        ),
    };
    let quote = quote_exact_input(quote_rpc_url, args.quoter, params, quote_block).await?;

    let gross_profit = if quote.amount_out >= args.amount_in {
        format!("{}", quote.amount_out - args.amount_in)
    } else {
        format!("-{}", args.amount_in - quote.amount_out)
    };

    println!("analysis_block={}", search.analysis_block);
    println!("quote_block={quote_block:?}");
    println!("proxy_pool_id={}", proxy_pool.pool_id);
    println!("reference_pool_id={}", reference.pool.pool_id); // первый найденный референс пул 
    println!("start_currency={}", args.start_currency);
    println!("amount_in={}", args.amount_in);
    println!("amount_out={}", quote.amount_out);
    println!("gross_profit={}", gross_profit);
    println!("quoter_gas_estimate={}", quote.gas_estimate);

    Ok(())
}
