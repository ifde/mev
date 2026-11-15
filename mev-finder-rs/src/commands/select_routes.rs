use anyhow::Result;

use crate::{cli::SelectRoutesArgs, select_routes::select_routes};

// CLI-обёртка: отбирает top-5 маршрутов из уже готовых результатов Anvil.
pub fn run(args: SelectRoutesArgs) -> Result<()> {
    let selected = select_routes(&args.run_dir)?;

    println!("selected_route_count={}", selected.selected_routes.len());
    println!("output={}", args.run_dir.join("selected-routes.json").display());
    Ok(())
}
