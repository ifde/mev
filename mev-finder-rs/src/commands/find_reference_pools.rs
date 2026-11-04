use anyhow::{Context, Result};

use crate::{
    cli::FindReferencePoolsArgs,
    proxy_pool::{find, ProxyPoolQuery},
    reference_search::{
        find_validated_reference_pools,
        ReferenceSearchRequest,
    },
};

pub async fn run(args: FindReferencePoolsArgs) -> Result<()> {
    let proxy_pool = find(ProxyPoolQuery {
        token_a: args.token_a,
        token_b: args.token_b,
    })
    .context("proxy pool fixture was not found for this token pair")?;

    let result = find_validated_reference_pools(ReferenceSearchRequest {
        subgraph_url: args.subgraph_url,
        api_key: args.api_key,
        rpc_url: args.rpc_url,
        pool_manager: args.pool_manager,
        state_view: args.state_view,
        analysis_block: args.analysis_block,
        proxy_pool,
    })
    .await?;

    println!("proxy_pool_id={}", proxy_pool.pool_id);
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