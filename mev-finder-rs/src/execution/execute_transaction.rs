// ==============================
// исполняет одну транзакция из плана
// fork Anvil, ReplaySwap, V4Quoter, кэш и JSON одной транзакции
// ============================== 

use std::{fs, path::PathBuf, time::Instant};

use alloy::{
    eips::BlockId,
    primitives::{
        Address, B256, Bytes, I256, U256,
        aliases::{I24, U24, U160},
        keccak256,
    },
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    sol_types::{SolCall, SolValue},
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    build_plan::EvaluationPlan,
    cache,
    chains::ethereum_mainnet::funding_holder_for_token,
    constants::ROUTE_GRID_PPM,
    gateways::{anvil::AnvilFork, erc20::ERC20, quoter::quote_exact_input},
    route::quote_exact_params,
    v4::{IStateView, PoolKey, ReplaySwap, Swap, SwapParams, TestSettings},
};

pub struct ExecuteTransactionRequest {
    pub plan: PathBuf,
    pub transaction_plan_index: usize,
    pub rpc_url: String,
    pub pool_manager: Address,
    pub state_view: Address,
    pub quoter: Address,
    pub swap_artifact: PathBuf,
    pub output: PathBuf,
}

#[derive(Serialize)]
struct EvaluationCacheKey {
    schema_version: u32,
    plan_hash: B256,
    transaction_plan_index: usize,
    proxy_transaction_hash: B256,
    swap_sender: Address,
    funding_holder: Option<Address>,
    pool_manager: Address,
    state_view: Address,
    quoter: Address,
    swap_artifact_hash: B256,
    route_grid_ppm: [u32; 3],
}

/* 
Наше приближение:
fork конца B − 1
=>  локальный deploy ReplaySwap
=>  локальный approve
=>  локальный proxy-swap
=>  локальный блок B
=>  V4Quoter на state после этого swap
*/
pub async fn execute_transaction(args: ExecuteTransactionRequest) -> Result<()> {
    let started = Instant::now();
    let plan_bytes =
        fs::read(&args.plan) // читаем plan.json, который создает оркестратор
            .with_context(|| format!("failed to read plan {}", args.plan.display()))?;
    let plan_hash = keccak256(&plan_bytes); // хешируем 
    let plan: EvaluationPlan = serde_json::from_slice(&plan_bytes)?; // десериализуем EvaluationPlan
    ensure!(
        plan.chain_id == 1,
        "only Ethereum mainnet plans are supported"
    );
    let entry = plan
        .transaction_plans
        .get(args.transaction_plan_index) // Достаем конкретный индекс транзакции
        .context("transaction_plan_index is outside transaction_plans")?;
    let tx = entry.proxy_transaction;
    let pool = plan.proxy_pool; // достаем прокси пул из плана, к которому и относится это транзакция 
    ensure!(pool.hooks == Address::ZERO, "proxy pool must have no hook");
    ensure!(
        tx.amount_in > 0,
        "proxy transaction amount must be positive"
    );
    let input_delta = if tx.zero_for_one {
        tx.amount0
    } else {
        tx.amount1
    };
    ensure!(
        input_delta < 0 && input_delta.unsigned_abs() == tx.amount_in,
        "proxy swap direction/amount disagrees with v4 event; rebuild the plan"
    );
    let input_token = if tx.zero_for_one {
        pool.currency0
    } else {
        pool.currency1
    };
    ensure!(
        input_token != Address::ZERO,
        "this replay supports ERC-20 input only"
    );
    let fork_block = tx
        .block_number
        .checked_sub(1) // безопасное вычитание единицы
        .context("block must be positive")?;

    // Эти значения были загружены вместе с ProxyTransaction при построении плана.
    let swap_sender = tx.transaction_sender;
    let funding_holder = funding_holder_for_token(input_token);

    let artifact_bytes = fs::read(&args.swap_artifact) // читаем скомпилированный байткод UniswapV4-роутера  
        .context("build ReplaySwap first: cd mev-foundry && forge build")?;
    let cache_key = EvaluationCacheKey {
        schema_version: 4,
        plan_hash, // сам хеш плана 
        transaction_plan_index: args.transaction_plan_index, // индекс транзакции из плана 
        proxy_transaction_hash: tx.transaction_hash, // хеш транзакции 
        swap_sender,
        funding_holder,
        pool_manager: args.pool_manager,
        state_view: args.state_view,
        quoter: args.quoter,
        swap_artifact_hash: keccak256(&artifact_bytes),
        route_grid_ppm: ROUTE_GRID_PPM,
    };
    let cache_key_json = serde_json::to_value(&cache_key)?;

    // если этот транзакцию уже прогоняли, то заново не делаем : достаем из кеша 
    if let Some(cached_result) = cache::read_json::<Value>(&args.output)? {
        if cached_result.get("cache_key") == Some(&cache_key_json) {
            println!("cache_hit=true");
            println!("output={}", args.output.display());
            return Ok(());
        }
    }

    // Скомпилированный готовый Uniswap swap (который компилируется в Foundry) 
    let artifact: Value = serde_json::from_slice(&artifact_bytes)?;
    let mut bytecode = artifact["bytecode"]["object"]
        .as_str()
        .context("artifact has no creation bytecode")?
        .parse::<Bytes>()?
        .to_vec();
    bytecode.extend(args.pool_manager.abi_encode());

    // печатает в stderr, а не в stdout
    eprintln!("starting fork at block {fork_block}"); // запускаем локальный Anvil и делаем своп 
    let fork = AnvilFork::start(&args.rpc_url, fork_block).await?;
    let provider = ProviderBuilder::new().connect_http(fork.rpc_url.parse()?); // локальный Anvil становится нашим провайдером 
    let view = IStateView::new(args.state_view, &provider);
    let before = view.getSlot0(pool.pool_id).call().await?;
    ensure!(
        !before.sqrtPriceX96.is_zero(),
        "proxy pool is not initialized at fork block"
    );
    let token = ERC20::new(input_token, &provider);
    let funding_holder = if let Some(holder) = funding_holder {
        holder
    } else {
        let result = json!({
            "cache_key": cache_key,
            "mode": "block_start_replay",
            "status": "skipped_unconfigured_funding_token",
            "proxy_transaction": tx,
            "fork_block": fork_block,
            "transaction_plan_index": args.transaction_plan_index,
            "swap_sender": swap_sender,
            "input_token": input_token,
            "elapsed_ms": started.elapsed().as_millis()
        });
        cache::write_json(&args.output, &result)?;
        println!("status=skipped_unconfigured_funding_token");
        println!("output={}", args.output.display());
        return Ok(());
    };
    let funding_balance = token.balanceOf(funding_holder).call().await?;
    if funding_balance < U256::from(tx.amount_in) {
        let result = json!({
            "cache_key": cache_key,
            "mode": "block_start_replay",
            "status": "skipped_insufficient_funding_balance",
            "proxy_transaction": tx,
            "fork_block": fork_block,
            "transaction_plan_index": args.transaction_plan_index,
            "swap_sender": swap_sender,
            "funding_holder": funding_holder,
            "input_token": input_token,
            "required_amount_in": tx.amount_in.to_string(),
            "available_funding_balance": funding_balance.to_string(),
            "elapsed_ms": started.elapsed().as_millis()
        });
        cache::write_json(&args.output, &result)?;
        println!("status=skipped_insufficient_funding_balance");
        println!("output={}", args.output.display());
        return Ok(());
    }

    let _: Value = provider
        .raw_request("anvil_impersonateAccount".into(), (swap_sender,))
        .await?;
    let _: Value = provider
        .raw_request("anvil_impersonateAccount".into(), (funding_holder,))
        .await?;
    let _: Value = provider
        .raw_request(
            "anvil_setBalance".into(),
            (swap_sender, U256::from(10u64).pow(U256::from(20))),
        )
        .await?;
    let _: Value = provider
        .raw_request(
            "anvil_setBalance".into(),
            (funding_holder, U256::from(10u64).pow(U256::from(20))),
        )
        .await?;
    let deployer = provider
        .get_accounts()
        .await?
        .into_iter()
        .next()
        .context("no Anvil account")?;
    ensure!(
        deployer != swap_sender,
        "swap_sender must differ from Anvil deployer"
    );
    let deploy_nonce = provider.get_transaction_count(deployer).await?;
    let funding_nonce = provider.get_transaction_count(funding_holder).await?;
    let swap_nonce = provider.get_transaction_count(swap_sender).await?;
    let router = deployer.create(deploy_nonce);
    let gas_price = tx.block_base_fee_per_gas * 2 + 1_000_000_000;
    let make_tx = |from, nonce, data| {
        TransactionRequest::default()
            .from(from)
            .nonce(nonce)
            .gas_limit(5_000_000)
            .gas_price(gas_price)
            .input(data)
    };

    // Все три транзакции попадут в один локальный блок B в порядке отправки.
    // Подготовка не сдвигает номер блока перед пользовательским swap.
    let deploy = provider // деплоим ReplaySwap.sol 
        .send_transaction(make_tx(
            deployer,
            deploy_nonce,
            Bytes::from(bytecode).into(),
        ))
        .await?;
    let funding_data = ERC20::transferCall {
        to: swap_sender,
        amount: U256::from(tx.amount_in),
    }
    .abi_encode();
    let funding = provider
        .send_transaction(
            make_tx(
                funding_holder,
                funding_nonce,
                Bytes::from(funding_data).into(),
            )
            .to(input_token),
        )
        .await?;
    let approve_data = ERC20::approveCall {
        spender: router, // это тот ReplaySwap.sol, который мы задеплоили 
        amount: U256::from(tx.amount_in),
    }
    .abi_encode();
    let approve = provider // делаем аппрув, что spender разрешает ReplaySwap.sol тратить свои ERC20 
        .send_transaction(
            make_tx(
                swap_sender,
                swap_nonce,
                Bytes::from(approve_data).into(),
            )
            .to(input_token),
        )
        .await?; 
        // делаем своп 
    let swap_data = ReplaySwap::swapCall {
        key: PoolKey {
            currency0: pool.currency0,
            currency1: pool.currency1,
            fee: U24::from(pool.fee),
            tickSpacing: I24::try_from(pool.tick_spacing)?,
            hooks: pool.hooks,
        },
        params: SwapParams {
            zeroForOne: tx.zero_for_one,
            amountSpecified: -I256::try_from(tx.amount_in)?,
            sqrtPriceLimitX96: if tx.zero_for_one {
                U160::from(4295128740u64)
            } else {
                "1461446703485210103287273052203988822378723970341".parse()?
            },
        },
        settings: TestSettings {
            takeClaims: false,
            settleUsingBurn: false,
        },
        hookData: Bytes::new(),
    }
    .abi_encode();
    let swap = provider
        .send_transaction(
            make_tx(
                swap_sender,
                swap_nonce + 1,
                Bytes::from(swap_data).into(),
            )
            .to(router),
        )
        .await?;
    let _: Value = provider
        .raw_request(
            "evm_setNextBlockTimestamp".into(),
            (tx.block_timestamp,),
        )
        .await?;
    let _: Value = provider
        .raw_request(
            "anvil_setNextBlockBaseFeePerGas".into(),
            (U256::from(tx.block_base_fee_per_gas),),
        )
        .await?;
    eprintln!("executing proxy swap in local block {}", tx.block_number);
    let _: Value = provider.raw_request("evm_mine".into(), ()).await?;
    let mut replayed_amounts = None;
    for (name, pending) in [
        ("deploy", deploy),
        ("funding", funding),
        ("approve", approve),
        ("swap", swap),
    ] {
        let receipt = provider
            .get_transaction_receipt(*pending.tx_hash())
            .await?
            .with_context(|| format!("{name} was not mined"))?;
        ensure!(receipt.status(), "{name} reverted");
        ensure!(
            receipt.block_number == Some(tx.block_number),
            "unexpected local block number"
        );
        if name == "swap" {
            for log in receipt.inner.logs() {
                if log.address() == args.pool_manager {
                    if let Ok(decoded) = log.log_decode_validate::<Swap>() {
                        let event = decoded.data();
                        if event.id == pool.pool_id {
                            replayed_amounts = Some((event.amount0, event.amount1));
                        }
                    }
                }
            }
        }
    }
    let (actual_amount0, actual_amount1) = replayed_amounts.context("proxy Swap event missing")?;
    let actual_input = if tx.zero_for_one {
        actual_amount0
    } else {
        actual_amount1
    };
    ensure!(
        actual_input < 0 && actual_input.unsigned_abs() == tx.amount_in,
        "proxy swap did not consume the full historical input"
    );
    let after = view.getSlot0(pool.pool_id).call().await?;
    let mut evaluations = Vec::new();
    for evaluation in &entry.route_evaluations {
        let route = plan
            .routes
            .iter()
            .find(|route| route.route_id == evaluation.route_id)
            .context("route_id from transaction plan is missing")?;

        for grid_ppm in ROUTE_GRID_PPM {
            let amount_in = evaluation
                .amount_anchor
                .checked_mul(u128::from(grid_ppm))
                .context("amount grid multiplication overflow")?
                / 1_000_000;
            if amount_in == 0 {
                evaluations.push(json!({
                    "route_id": route.route_id,
                    "route": route.route,
                    "grid_ppm": grid_ppm,
                    "status": "skipped_zero_amount"
                }));
                continue;
            }

            let quote_started = Instant::now();
            let quote_result = quote_exact_input(
                &fork.rpc_url,
                args.quoter,
                quote_exact_params(&route.route, amount_in)?,
                BlockId::latest(),
            )
            .await;

            match quote_result {
                Ok(quote) => {
                    let profit = I256::try_from(quote.amount_out)? - I256::try_from(amount_in)?;
                    evaluations.push(json!({
                        "route_id": route.route_id,
                        "route": route.route,
                        "grid_ppm": grid_ppm,
                        "amount_in": amount_in.to_string(),
                        "amount_out": quote.amount_out.to_string(),
                        "gross_profit": profit.to_string(),
                        "quoter_gas_estimate": quote.gas_estimate,
                        "quote_elapsed_ms": quote_started.elapsed().as_millis(),
                        "status": "ok"
                    }));
                }
                Err(error) => evaluations.push(json!({
                    "route_id": route.route_id,
                    "route": route.route,
                    "grid_ppm": grid_ppm,
                    "amount_in": amount_in.to_string(),
                    "quote_elapsed_ms": quote_started.elapsed().as_millis(),
                    "status": "quote_error",
                    "error": format!("{error:#}")
                })),
            }
        }
    }
    let unchanged = view.getSlot0(pool.pool_id).call().await?;
    ensure!(
        after.sqrtPriceX96 == unchanged.sqrtPriceX96 && after.tick == unchanged.tick,
        "Quoter changed proxy state"
    );
    let result = json!({
        "cache_key": cache_key,
        "mode": "block_start_replay", "proxy_transaction": tx, "fork_block": fork_block,
        "transaction_plan_index": args.transaction_plan_index,
        "swap_sender": swap_sender,
        "funding_holder": funding_holder,
        "funding_amount": tx.amount_in.to_string(),
        "sqrt_price_before": before.sqrtPriceX96.to_string(),
        "sqrt_price_after": after.sqrtPriceX96.to_string(),
        "replayed_amount0": actual_amount0.to_string(), "replayed_amount1": actual_amount1.to_string(),
        "evaluations": evaluations,
        "elapsed_ms": started.elapsed().as_millis()
    });
    cache::write_json(&args.output, &result)?;
    println!(
        "evaluations={}",
        result["evaluations"].as_array().map_or(0, Vec::len)
    );
    println!("output={}", args.output.display());
    Ok(())
}
