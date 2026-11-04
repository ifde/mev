use anyhow::{Context, Result};

use crate::{
    cli::QuoteTwoPoolCycleArgs,
    proxy_pool::{find, ProxyPoolQuery},
    quoter::quote_exact_input,
    reference_search::{
        find_validated_reference_pools,
        ReferenceSearchRequest,
    },
    route::two_pool_cycle,
};

pub async fn run(args: QuoteTwoPoolCycleArgs) -> Result<()> {
    let search = args.reference_search;

    // находим прокси пул 
    let proxy_pool = find(ProxyPoolQuery {
        token_a: search.token_a,
        token_b: search.token_b,
    })
    .context("proxy pool fixture was not found for this token pair")?;

    // находим референс пулы 
    let result = find_validated_reference_pools(ReferenceSearchRequest {
        subgraph_url: search.subgraph_url,
        api_key: search.api_key,
        rpc_url: search.rpc_url.clone(), // клонируем, так как далее будем еще раз использовать это поле 
        pool_manager: search.pool_manager,
        state_view: search.state_view,
        analysis_block: search.analysis_block,
        proxy_pool,
    })
    .await?;

    // Достаем только первый референс пул (потом добавим больше)  
    let reference = result
        .pools
        .first()
        .context("no validated reference pool was found")?;

    // Создаем параметры маршрута  
    let params = two_pool_cycle(
        &proxy_pool,
        &reference.pool,
        args.start_currency,
        args.amount_in,
    )?;

    // вызываем V4Quoter для проверки маршрута 
    let quote = quote_exact_input(
        &search.rpc_url,
        args.quoter,
        params,
        search.analysis_block,
    )
    .await?;

    let gross_profit = if quote.amount_out >= args.amount_in {
        format!("{}", quote.amount_out - args.amount_in)
    } else {
        format!("-{}", args.amount_in - quote.amount_out)
    };

    println!("analysis_block={}", search.analysis_block);
    println!("proxy_pool_id={}", proxy_pool.pool_id);
    println!("reference_pool_id={}", reference.pool.pool_id); // первый найденный референс пул 
    println!("start_currency={}", args.start_currency);
    println!("amount_in={}", args.amount_in);
    println!("amount_out={}", quote.amount_out);
    println!("gross_profit={}", gross_profit);
    println!("quoter_gas_estimate={}", quote.gas_estimate);

    Ok(())
}