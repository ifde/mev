// ==============================
// Возвращает вектор найденных маршрутов 
// Сейчас ищет только пути из двух пулов (наш таргет пул и один референс пул). 
// Для каждого пути возвращается 4 маршрута: 2 направления (прямое и обратное) x входной токен (token0 / token1) 
// ============================== 

use alloy::{
    primitives::{
        aliases::{I24, U24},
        Address,
        B256,
        Bytes,
    },
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    proxy_pool::ProxyPool,
    reference_pool::ReferencePool,
    v4::{PathKey, QuoteExactParams},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteHop {
    pub pool_id: B256,
    pub currency0: Address,
    pub currency1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Route {
    pub kind: RouteKind,
    pub start_currency: Address,
    pub hops: Vec<RouteHop>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteKind {
    Direct,
    Reverse,
}

impl From<&ProxyPool> for RouteHop {
    fn from(pool: &ProxyPool) -> Self {
        Self {
            pool_id: pool.pool_id,
            currency0: pool.currency0,
            currency1: pool.currency1,
            fee: pool.fee,
            tick_spacing: pool.tick_spacing,
            hooks: pool.hooks,
        }
    }
}

impl From<&ReferencePool> for RouteHop {
    fn from(pool: &ReferencePool) -> Self {
        Self {
            pool_id: pool.pool_id,
            currency0: pool.currency0,
            currency1: pool.currency1,
            fee: pool.fee,
            tick_spacing: pool.tick_spacing,
            hooks: pool.hooks,
        }
    }
}

// Для каждого reference-пула строим все четыре статические топологии.
// Выбор двух маршрутов для конкретного user swap делает orchestrator.
pub fn find_two_pool_routes(
    proxy: &ProxyPool,
    references: &[ReferencePool],
) -> Result<Vec<Route>> {
    let mut routes = Vec::with_capacity(references.len() * 4);

    for reference in references {
        if proxy.pool_id == reference.pool_id {
            bail!("proxy pool and reference pool must be different");
        }

        if proxy.currency0 != reference.currency0
            || proxy.currency1 != reference.currency1
        {
            bail!("proxy pool and reference pool must have the same token pair");
        }

        routes.push(Route {
            kind: RouteKind::Direct,
            start_currency: proxy.currency0,
            hops: vec![RouteHop::from(proxy), RouteHop::from(reference)],
        });
        routes.push(Route {
            kind: RouteKind::Direct,
            start_currency: proxy.currency1,
            hops: vec![RouteHop::from(proxy), RouteHop::from(reference)],
        });
        routes.push(Route {
            kind: RouteKind::Reverse,
            start_currency: proxy.currency0,
            hops: vec![RouteHop::from(reference), RouteHop::from(proxy)],
        });
        routes.push(Route {
            kind: RouteKind::Reverse,
            start_currency: proxy.currency1,
            hops: vec![RouteHop::from(reference), RouteHop::from(proxy)],
        });
    }

    Ok(routes)
}

pub fn quote_exact_params(
    route: &Route,
    amount_in: u128,
) -> Result<QuoteExactParams> {
    if amount_in == 0 {
        bail!("amount_in must be greater than zero");
    }

    if route.hops.len() < 2 {
        bail!("route must contain at least two pools");
    }

    let mut current_currency = route.start_currency;
    let mut path = Vec::with_capacity(route.hops.len());

    for (index, hop) in route.hops.iter().enumerate() {
        let intermediate_currency = if current_currency == hop.currency0 {
            hop.currency1
        } else if current_currency == hop.currency1 {
            hop.currency0
        } else {
            bail!("route hop {index} does not accept the previous currency");
        };

        let tick_spacing = I24::try_from(hop.tick_spacing)
            .context("route tick_spacing does not fit Solidity int24")?;

        path.push(PathKey {
            intermediateCurrency: intermediate_currency,
            fee: U24::from(hop.fee),
            tickSpacing: tick_spacing,
            hooks: hop.hooks,
            hookData: Bytes::new(),
        });

        current_currency = intermediate_currency;
    }

    if current_currency != route.start_currency {
        bail!("route does not end in start_currency");
    }

    Ok(QuoteExactParams {
        exactCurrency: route.start_currency,
        exactAmount: amount_in,
        path,
    })
}
