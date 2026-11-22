use anyhow::Result;

use crate::{
    cli::ExecuteTransactionArgs,
    execution::execute_transaction::{ExecuteTransactionRequest, execute_transaction},
};

// CLI-обёртка: переводит аргументы команды в запрос модуля evaluation.
pub async fn run(args: ExecuteTransactionArgs) -> Result<()> {
    execute_transaction(ExecuteTransactionRequest {
        plan: args.plan,
        transaction_plan_index: args.transaction_plan_index,
        rpc_url: args.rpc_url,
        pool_manager: args.pool_manager,
        state_view: args.state_view,
        quoter: args.quoter,
        swap_artifact: args.swap_artifact,
        output: args.output,
    })
    .await
}
