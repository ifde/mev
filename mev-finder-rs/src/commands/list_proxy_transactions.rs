use anyhow::Result;

use crate::{
    cli::ListProxyTransactionsArgs,
    constants::{MAX_PROXY_POOL_CANDIDATES, MAX_PROXY_TRANSACTIONS},
    find_proxy_pool::{FindProxyPoolRequest, find_proxy_pool},
    find_proxy_transactions::{LoadProxyTransactionsRequest, load_proxy_transactions},
};

pub async fn run(args: ListProxyTransactionsArgs) -> Result<()> {
    let proxy = find_proxy_pool(FindProxyPoolRequest {
        subgraph_url: args.subgraph_url,
        api_key: args.api_key,
        rpc_url: args.rpc_url.clone(),
        state_view: args.state_view,
        analysis_block: args.analysis_block,
        token_a: args.token_a,
        token_b: args.token_b,
        proxy_pool_id: None,
        max_candidates: MAX_PROXY_POOL_CANDIDATES,
    })
    .await?;
    let proxy_pool = proxy.pool;

    let loaded = load_proxy_transactions(LoadProxyTransactionsRequest {
        rpc_url: args.rpc_url,
        expected_chain_id: 1,
        pool_manager: args.pool_manager,
        proxy_pool,
        from_block: args.from_block,
        to_block: args.to_block,
        max_transactions: MAX_PROXY_TRANSACTIONS,
    })
    .await?;

    println!("proxy_pool_id={}", proxy_pool.pool_id);
    println!("proxy_pool_cache_hit={}", proxy.cache_hit);
    println!("cache_hit={}", loaded.cache_hit);
    println!("swap_logs_found={}", loaded.found_log_count);
    println!(
        "proxy_transactions_found={}",
        loaded.found_transaction_count
    );
    println!(
        "proxy_transactions_skipped_multi_swap={}",
        loaded.skipped_multi_swap_transaction_count,
    );
    println!("proxy_blocks_selected={}", loaded.selected_block_count);
    println!("proxy_transactions_selected={}", loaded.transactions.len());
    println!(
        "top_10_max_transaction_index={}",
        loaded
            .top_10_max_transaction_index
            .map(|index| index.to_string())
            .unwrap_or_else(|| "none".to_owned()),
    );

    for transaction in loaded.transactions {
        println!(
            "block={} tx_index={} tx={} log_index={} zero_for_one={} amount_in={} amount0={} amount1={}",
            transaction.block_number,
            transaction.transaction_index,
            transaction.transaction_hash,
            transaction.log_index,
            transaction.zero_for_one,
            transaction.amount_in,
            transaction.amount0,
            transaction.amount1,
        );
    }

    Ok(())
}
