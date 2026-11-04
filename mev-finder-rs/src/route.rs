// ==============================
// Возвращает QuoteExactParams (структура, которая хранит маршрут для проверки через V4Quoter) 
// ============================== 

use alloy::{
    primitives::{
        aliases::{I24, U24},
        Address,
        Bytes,
    },
};
use anyhow::{bail, Context, Result};

use crate::{
    proxy_pool::ProxyPool,
    reference_pool::ReferencePool,
    v4::{PathKey, QuoteExactParams},
};


pub fn two_pool_cycle(
    proxy: &ProxyPool,
    reference: &ReferencePool,
    start_currency: Address,
    amount_in: u128,
) -> Result<QuoteExactParams> {
    if amount_in == 0 {
        bail!("amount_in must be greater than zero");
    }

    if proxy.pool_id == reference.pool_id {
        bail!("proxy pool and reference pool must be different");
    }

    if proxy.currency0 != reference.currency0
        || proxy.currency1 != reference.currency1
    {
        bail!("proxy pool and reference pool must have the same token pair");
    }

    let intermediate_currency = if start_currency == proxy.currency0 {
        proxy.currency1
    } else if start_currency == proxy.currency1 {
        proxy.currency0
    } else {
        bail!("start_currency is not in the proxy pool");
    };

    let proxy_tick_spacing = I24::try_from(proxy.tick_spacing)
        .context("proxy tick_spacing does not fit Solidity int24")?;

    let reference_tick_spacing = I24::try_from(reference.tick_spacing)
        .context("reference tick_spacing does not fit Solidity int24")?;

    Ok(QuoteExactParams {
        exactCurrency: start_currency,
        exactAmount: amount_in,
        path: vec![
            PathKey {
                intermediateCurrency: intermediate_currency,
                fee: U24::from(proxy.fee),
                tickSpacing: proxy_tick_spacing,
                hooks: proxy.hooks,
                hookData: Bytes::new(),
            },
            PathKey {
                intermediateCurrency: start_currency,
                fee: U24::from(reference.fee),
                tickSpacing: reference_tick_spacing,
                hooks: reference.hooks,
                hookData: Bytes::new(),
            },
        ],
    })
}