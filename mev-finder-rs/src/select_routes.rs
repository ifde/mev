// ==============================
// Отбирает top-5 прибыльных маршрутов из результатов одного запуска.
// Ничего не запрашивает у RPC и не запускает Anvil.
// ==============================

use std::{collections::HashMap, fs, path::Path};

use alloy::primitives::{B256, I256, keccak256};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::{
    cache,
    find_proxy_transactions::ProxyTransaction,
    route::Route,
};

const SELECTED_ROUTE_LIMIT: usize = 5;

// это тот JSON result, который сохраняет модуль execute_transaction.rs 
// там это просто JSON хардкод 
// поэтому здесь определяем, как это размаппить в структуру TransactionResult. 
// поэтому и нужна обертка (атрибут) Deserialize 
#[derive(Deserialize)]
struct TransactionResult {
    proxy_transaction: ProxyTransaction,
    evaluations: Vec<RouteEvaluationResult>,
}

#[derive(Deserialize)]
struct RouteEvaluationResult {
    route_id: usize,
    route: Route,
    grid_ppm: u32,
    status: String,
    amount_in: Option<String>,
    amount_out: Option<String>,
    gross_profit: Option<String>,
    quoter_gas_estimate: Option<u64>,
}

// результат функции 
#[derive(Deserialize, Serialize)]
pub struct SelectedRoutes {
    pub schema_version: u32,
    pub source_plan_hash: B256,
    pub selected_routes: Vec<SelectedRoute>,
}

#[derive(Deserialize, Serialize)]
pub struct SelectedRoute {
    pub route_id: usize,
    pub route: Route,
    // Наблюдение, на котором этот маршрут показал свой максимум.
    pub source_proxy_transaction: ProxyTransaction,
    pub grid_ppm: u32,
    pub amount_in: String,
    pub amount_out: String,
    pub gross_profit: String, // это строковое значение! не числовое 
    pub quoter_gas_estimate: u64,
}

// структура кандидата нам нужна, чтобы иметь gross_profit как числовое значение 
struct Candidate {
    selected: SelectedRoute,
    gross_profit: I256,
}

// run_dir - это основая папка проверка конкретного пула на промежутке блоков 
// она создается еще в самом начале оркестратором 
// условно называется usdc-weth-{block_from}-{block_to} 
pub fn select_routes(run_dir: &Path) -> Result<SelectedRoutes> {
    let results_dir = run_dir.join("results");
    let plan_bytes = fs::read(run_dir.join("plan.json"))
        .with_context(|| format!("failed to read plan in {}", run_dir.display()))?;
    let mut best_by_route = HashMap::<usize, Candidate>::new(); // пары (id маршрута, кадидат - тот же маршрут с его текущим найденным gross_profit) 

    // читаем все транзакции 
    for entry in fs::read_dir(&results_dir)
        .with_context(|| format!("failed to read results directory {}", results_dir.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }

        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read result {}", path.display()))?;
        let transaction_result: TransactionResult = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to decode result {}", path.display()))?;

        // для каждой транзакции идем по всем результатам хопов 
        for evaluation in transaction_result.evaluations {
            if evaluation.status != "ok" {
                continue;
            }

            // достаем результаты маршруты (только если все они есть) 
            // gross_profit можем не записать, если он и так нулевой 
            let (amount_in, amount_out, gross_profit, quoter_gas_estimate) = match (
                evaluation.amount_in,
                evaluation.amount_out,
                evaluation.gross_profit,
                evaluation.quoter_gas_estimate,
            ) {
                (Some(amount_in), Some(amount_out), Some(gross_profit), Some(gas)) => {
                    (amount_in, amount_out, gross_profit, gas)
                }
                _ => continue,
            };
            let gross_profit_value = gross_profit
                .parse::<I256>()
                .with_context(|| format!("invalid gross_profit in {}", path.display()))?;
            if gross_profit_value <= I256::ZERO {
                continue;
            }

            let candidate = Candidate {
                gross_profit: gross_profit_value, // тут важно, что мы это храним в int, чтобы сравнивать числа 
                selected: SelectedRoute {
                    route_id: evaluation.route_id,
                    route: evaluation.route,
                    source_proxy_transaction: transaction_result.proxy_transaction,
                    grid_ppm: evaluation.grid_ppm,
                    amount_in,
                    amount_out,
                    gross_profit,
                    quoter_gas_estimate,
                },
            };

            let should_replace = best_by_route
                .get(&candidate.selected.route_id) // находим текущий максимальный profit для текущего маршрута 
                .is_none_or(|best| candidate.gross_profit > best.gross_profit); // Метод для Option<T>. Возвращает true, если значение равно None или значение равно Some(x) и замыкание(x) == true 
            if should_replace {
                best_by_route.insert(candidate.selected.route_id, candidate);
            }
        }
    }

    // Отбираем топ самых прибыльных маршрутов 
    let mut candidates = best_by_route.into_values().collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| (std::cmp::Reverse(candidate.gross_profit), candidate.selected.route_id));
    let selected_routes = candidates
        .into_iter()
        .take(SELECTED_ROUTE_LIMIT)
        .map(|candidate| candidate.selected)
        .collect::<Vec<_>>();

    ensure!(
        !selected_routes.is_empty(),
        "no positive route evaluations in {}",
        results_dir.display()
    );

    let selected = SelectedRoutes {
        schema_version: 1,
        source_plan_hash: keccak256(plan_bytes),
        selected_routes,
    };
    cache::write_json(&run_dir.join("selected-routes.json"), &selected)?;
    Ok(selected)
}
