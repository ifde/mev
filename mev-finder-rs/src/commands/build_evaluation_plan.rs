use std::fs;

use anyhow::{Context, Result};

use crate::{
    cli::BuildEvaluationPlanArgs,
    constants::MAX_PROXY_TRANSACTIONS,
    orchestrator::{build_evaluation_plan, BuildEvaluationPlanRequest},
};

pub async fn run(args: BuildEvaluationPlanArgs) -> Result<()> {
    let search = args.reference_search;
    let plan = build_evaluation_plan(BuildEvaluationPlanRequest {
        subgraph_url: search.subgraph_url,
        api_key: search.api_key,
        rpc_url: search.rpc_url,
        expected_chain_id: 1,
        pool_manager: args.pool_manager,
        state_view: search.state_view,
        analysis_block: search.analysis_block,
        token_a: search.token_a,
        token_b: search.token_b,
        from_block: args.from_block,
        to_block: args.to_block,
        max_transactions: MAX_PROXY_TRANSACTIONS,
    })
    .await?;

    let contents = serde_json::to_string_pretty(&plan)
        .context("failed to encode evaluation plan")?;
    fs::write(&args.output, contents)
        .with_context(|| format!("failed to write {}", args.output))?;

    println!("output={}", args.output);
    println!("routes={}", plan.routes.len());
    println!("transaction_plans={}", plan.transaction_plans.len());
    println!(
        "route_evaluations={}",
        plan.transaction_plans
            .iter()
            .map(|plan| plan.route_evaluations.len())
            .sum::<usize>(),
    );

    Ok(())
}
