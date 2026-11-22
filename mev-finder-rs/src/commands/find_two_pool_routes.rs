use anyhow::Result;

use crate::{
    cli::FindTwoPoolRoutesArgs,
    constants::{MAX_PROXY_POOL_CANDIDATES, MAX_REFERENCE_POOL_CANDIDATES},
    find_proxy_pool::{FindProxyPoolRequest, find_proxy_pool},
    find_reference_pools::{ReferenceSearchRequest, find_validated_reference_pools},
    route::find_two_pool_routes,
};

pub async fn run(args: FindTwoPoolRoutesArgs) -> Result<()> {
    let search = args.reference_search;

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

    let references = find_validated_reference_pools(ReferenceSearchRequest {
        subgraph_url: search.subgraph_url,
        api_key: search.api_key,
        rpc_url: search.rpc_url,
        state_view: search.state_view,
        analysis_block: search.analysis_block,
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

    println!("proxy_pool_id={}", proxy.pool.pool_id);
    println!("reference_pools_validated={}", reference_pools.len());
    println!("two_pool_routes_found={}", routes.len());

    for (index, route) in routes.iter().enumerate() {
        let pool_ids = route
            .hops
            .iter()
            .map(|hop| hop.pool_id.to_string())
            .collect::<Vec<_>>()
            .join(" -> ");

        println!(
            "route={} kind={:?} start_currency={} pools={}",
            index, route.kind, route.start_currency, pool_ids,
        );
    }

    Ok(())
}
