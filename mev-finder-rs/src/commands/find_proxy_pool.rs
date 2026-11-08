use anyhow::Result;

use crate::{
    cli::FindProxyPoolArgs,
    constants::MAX_PROXY_POOL_CANDIDATES,
    proxy_pool::{find_proxy_pool, FindProxyPoolRequest},
};

pub async fn run(args: FindProxyPoolArgs) -> Result<()> {
    let result = find_proxy_pool(FindProxyPoolRequest {
        subgraph_url: args.subgraph_url,
        api_key: args.api_key,
        rpc_url: args.rpc_url,
        state_view: args.state_view,
        analysis_block: args.analysis_block,
        token_a: args.token_a,
        token_b: args.token_b,
        max_candidates: MAX_PROXY_POOL_CANDIDATES,
    })
    .await?;

    println!("cache_hit={}", result.cache_hit);
    println!("proxy_pools_discovered={}", result.discovered_count);
    println!("proxy_pool_id={}", result.pool.pool_id);
    println!("currency0={}", result.pool.currency0);
    println!("currency1={}", result.pool.currency1);
    println!("fee={}", result.pool.fee);
    println!("tick_spacing={}", result.pool.tick_spacing);
    println!("active_liquidity={}", result.state.active_liquidity);

    Ok(())
}
