

use anyhow::{bail, Result};

use crate::{
    cli::ListProxySwapsArgs,
    proxy_pool::{find, ProxyPoolQuery},
    proxy_swaps::{load_proxy_swaps, LoadProxySwapsRequest},
};

pub async fn run(args: ListProxySwapsArgs) -> Result<()> {
    let proxy_pool = match find(ProxyPoolQuery {
        token_a: args.token_a,
        token_b: args.token_b,
    }) {
        Some(pool) => pool,
        None => bail!("proxy pool fixture was not found for this token pair"),
    };

    let loaded = load_proxy_swaps(LoadProxySwapsRequest {
        rpc_url: args.rpc_url,
        pool_manager: args.pool_manager,
        proxy_pool,
        from_block: args.from_block,
        to_block: args.to_block,
    })
    .await?;

    println!("proxy_pool_id={}", proxy_pool.pool_id);
    println!("swap_logs_found={}", loaded.found_log_count);
    println!("swap_logs_selected={}", loaded.swaps.len());

    for swap in loaded.swaps {
        println!(
            "block={} tx_index={} tx={} log_index={} zero_for_one={} amount_in={}",
            swap.block_number, 
            swap.transaction_index, 
            swap.transaction_hash,
            swap.log_index,
            swap.zero_for_one,
            swap.amount_in,
        );
    }

    Ok(())
}