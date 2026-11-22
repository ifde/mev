// ==============================
// Создает Anvil Instance
// ==============================

use alloy::node_bindings::{Anvil, AnvilInstance};
use anyhow::{Context, Result};

// Отдельный локальный процесс запуска Anvil.
pub struct AnvilFork {
    // Alloy закрывает процесс, когда AnvilInstance удаляется.
    _instance: AnvilInstance,
    pub rpc_url: String,
}

impl AnvilFork {
    pub async fn start(upstream_rpc: &str, block: u64) -> Result<Self> {
        let anvil = Anvil::new()
            .host("127.0.0.1")
            .port(8546u16)
            .fork(upstream_rpc)
            .fork_block_number(block)
            .chain_id(1)
            .timeout(60_000)
            .args([
                // Общий disk cache между отдельными Anvil-процессами.
                // Каждый proxy-swap запускает новый fork, поэтому без этого пути
                // одинаковые state-слоты снова запрашиваются у upstream RPC.
                "--cache-path",
                "cache/anvil",
                "--compute-units-per-second",
                "50",
                "--no-mining",
                "--order",
                "fifo",
            ]);

        // try_spawn запускает процесс и синхронно ждет, пока Anvil напишет, что начал слушать порт.
        // Чтобы не блокировать основной рантайм Tokio, переносим это в spawn_blocking
        // Тогда основной рантайм не блокируется и Tokio может выполнять параллельные запросы выше по дереву
        let instance = tokio::task::spawn_blocking(move || anvil.try_spawn())
            .await
            .context("Anvil startup task failed")?
            .context("failed to start anvil")?;

        Ok(Self {
            rpc_url: instance.endpoint(),
            _instance: instance,
        })
    }
}
