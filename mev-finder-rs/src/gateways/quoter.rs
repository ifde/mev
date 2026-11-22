// ==============================
// Функция quote_exact_input()
// Выполняет вызов V4Quoter через eth_call() на указанном блоке или latest локального Anvil
// Параметры - маршрут Quoter
// ==============================

use alloy::{
    eips::BlockId,
    primitives::Address,
    providers::{Provider, ProviderBuilder},
};
use anyhow::{Context, Result, bail};

use crate::v4::{IV4Quoter, QuoteExactParams};

pub struct Quote {
    pub amount_out: u128,
    pub gas_estimate: u64,
}

pub async fn quote_exact_input(
    rpc_url: &str,
    quoter_address: Address,
    params: QuoteExactParams,
    block: BlockId,
) -> Result<Quote> {
    let rpc_url = rpc_url
        .parse()
        .context("rpc_url must be a valid HTTP URL")?;

    let provider = ProviderBuilder::new().connect_http(rpc_url);

    let chain_id = provider
        .get_chain_id()
        .await
        .context("eth_chainId request failed")?;

    if chain_id != 1 {
        bail!("expected Ethereum Mainnet chain_id=1, got {chain_id}");
    }

    let quoter = IV4Quoter::new(quoter_address, provider);

    let result = quoter
        .quoteExactInput(params)
        .block(block)
        .gas(1_000_000)
        .call()
        .await
        .context("V4Quoter.quoteExactInput failed")?;

    Ok(Quote {
        amount_out: u128::try_from(result.amountOut)
            .context("quoted amount_out does not fit u128")?,
        gas_estimate: u64::try_from(result.gasEstimate)
            .context("quoted gas_estimate does not fit u64")?,
    })
}
