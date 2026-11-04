use anyhow::{bail, Result};

use crate::{
    cli::FindProxyPoolArgs,
    proxy_pool::{find, ProxyPoolQuery},
};

pub fn run(args: FindProxyPoolArgs) -> Result<()> {
    let query = ProxyPoolQuery {
        token_a: args.token_a,
        token_b: args.token_b,
    };

    match find(query) {
        Some(pool) => {
            println!("proxy_pool_id={}", pool.pool_id);
            println!("currency0={}", pool.currency0);
            println!("currency1={}", pool.currency1);

            Ok(())
        }
        None => bail!("proxy pool fixture was not found for this token pair"),
    }
}