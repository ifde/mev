// ==============================
// принимает на вход evaluation plan, который создается модулем build_plan.rs
// Далее исполняет все маршруты из этого плана 
// Для каждой транзакци под капотом вызывается execute_transaction.rs 
// ==============================

use std::{fs, path::PathBuf};

use alloy::primitives::{Address, B256, keccak256};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::Value;

use crate::{build_plan::EvaluationPlan, cache, constants::ROUTE_GRID_PPM};

use super::execute_transaction::{ExecuteTransactionRequest, execute_transaction};

pub struct ExecutePlanRequest {
    pub plan: PathBuf, // вот это основной аргумент : сам план 
    pub output_dir: PathBuf, // сохраняем результат в /results
    pub rpc_url: String,
    pub pool_manager: Address,
    pub state_view: Address,
    pub quoter: Address,
    pub swap_artifact: PathBuf, // путь к компилированному смарт-контракт ReplaySwap.sol 
                                // он создается через forge build 
}

#[derive(Serialize)]
struct EvaluationPlanRunConfig {
    schema_version: u32,
    plan_hash: B256,
    pool_manager: Address,
    state_view: Address,
    quoter: Address,
    route_grid_ppm: [u32; 3], // сетка цен 
}

pub async fn execute_plan(args: ExecutePlanRequest) -> Result<()> {
    let plan_bytes = fs::read(&args.plan) // имеет тип Vec<u8> (то есть, массив байтов) 
        .with_context(|| format!("failed to read plan {}", args.plan.display()))?;
    let plan: EvaluationPlan = // прочитать JSON из ссылки на непрерывный участок байтов 
        serde_json::from_slice(&plan_bytes).context("failed to decode evaluation plan")?;
    ensure!(
        !plan.transaction_plans.is_empty(),
        "evaluation plan has no transaction_plans"
    );

    // создаем конфиг исполнения плана транзакций: те адреса, которые используем, и Grid 
    let config = EvaluationPlanRunConfig {
        schema_version: 1,
        plan_hash: keccak256(&plan_bytes),
        pool_manager: args.pool_manager,
        state_view: args.state_view,
        quoter: args.quoter,
        route_grid_ppm: ROUTE_GRID_PPM,
    };
        // сохраняем в config.json 
    let config_json = serde_json::to_value(&config)?;
    let config_path = args.output_dir.join("config.json");

    if let Some(existing_config) = cache::read_json::<Value>(&config_path)? {
        ensure!(
            existing_config == config_json,
            "output_dir already contains results for a different evaluation configuration"
        );
    } else { 
        // если output dir не совпадает с plan path, то записываем plan и в эту новую директорию 
        cache::write_bytes(&args.output_dir.join("plan.json"), &plan_bytes)?; 
        cache::write_json(&config_path, &config)?; // записываем config.json 
    }

    // перебираем все транзакции из плана 
    for (transaction_plan_index, transaction_plan) in plan.transaction_plans.iter().enumerate() {
        let transaction_hash = transaction_plan.proxy_transaction.transaction_hash;
        let output = args // путь к файлу, в который будет записываться результат транзакции 
            .output_dir
            .join("results")
            .join(format!("{transaction_hash}.json"));

        eprintln!(
            "evaluating transaction_plan_index={transaction_plan_index} tx={transaction_hash}"
        ); 
        // исполняем транзакцию 
        execute_transaction(ExecuteTransactionRequest {
            plan: args.plan.clone(),
            transaction_plan_index,
            rpc_url: args.rpc_url.clone(),
            pool_manager: args.pool_manager,
            state_view: args.state_view,
            quoter: args.quoter,
            swap_artifact: args.swap_artifact.clone(),
            output,
        })
        .await?;
    }

    println!("output_dir={}", args.output_dir.display());
    println!("transaction_plans={}", plan.transaction_plans.len());
    Ok(())
}
