use anyhow::Result;

use crate::{
    cli::FindReferencePoolsArgs,
    constants::{MAX_PROXY_POOL_CANDIDATES, MAX_REFERENCE_POOL_CANDIDATES},
    proxy_pool::{find_proxy_pool, FindProxyPoolRequest},
    reference_search::{
        find_validated_reference_pools,
        ReferenceSearchRequest,
    },
};

pub async fn run(args: FindReferencePoolsArgs) -> Result<()> {
    let proxy = find_proxy_pool(FindProxyPoolRequest {
        subgraph_url: args.subgraph_url.clone(),
        api_key: args.api_key.clone(),
        rpc_url: args.rpc_url.clone(),
        state_view: args.state_view,
        analysis_block: args.analysis_block,
        token_a: args.token_a,
        token_b: args.token_b,
        max_candidates: MAX_PROXY_POOL_CANDIDATES,
    })
    .await?;
    let proxy_pool = proxy.pool;

    let result = find_validated_reference_pools(ReferenceSearchRequest {
        subgraph_url: args.subgraph_url,
        api_key: args.api_key,
        rpc_url: args.rpc_url,
        state_view: args.state_view,
        analysis_block: args.analysis_block,
        proxy_pool,
        max_reference_pools: MAX_REFERENCE_POOL_CANDIDATES,
    })
    .await?;

    println!("proxy_pool_id={}", proxy_pool.pool_id);
    println!("proxy_pool_cache_hit={}", proxy.cache_hit);
    println!("reference_pools_cache_hit={}", result.cache_hit);
    println!("reference_pools_discovered={}", result.discovered_count);
    println!("reference_pools_validated={}", result.pools.len());

    for (rank, candidate) in result.pools.iter().enumerate() {
        let pool = &candidate.pool;
        let state = &candidate.state;

        println!(
            "rank={} pool_id={} fee={} tick_spacing={} sqrt_price_x96={} tick={} active_liquidity={}
            tvl_usd={} tx_count={}",
            rank + 1,
            pool.pool_id,
            pool.fee,
            pool.tick_spacing,
            state.sqrt_price_x96,
            state.tick,
            state.active_liquidity,
            pool.total_value_locked_usd,
            pool.tx_count,
        );
    }

    Ok(())
}
