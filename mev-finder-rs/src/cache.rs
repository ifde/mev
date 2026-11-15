// ==============================
// Кеширует результаты работы функции
// cache::load_or_compute("component-version", &cache_key, || async { expensive_calculation().await })
// Используется для find_proxy_pool, find_validated_reference_pools и load_proxy_transactions
//
// ==============================

use std::{
    fs,
    future::Future,
    path::{Path, PathBuf},
};

use alloy::primitives::keccak256;
use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};

const CACHE_ROOT: &str = "cache";

pub struct CacheResult<Value> {
    pub value: Value,
    pub cache_hit: bool,
}

pub async fn load_or_compute<Request, Value, Compute, ComputeFuture>(
    component: &str,
    request: &Request,
    compute: Compute,
) -> Result<CacheResult<Value>>
where
    Request: Serialize,
    Value: Serialize + DeserializeOwned,
    Compute: FnOnce() -> ComputeFuture,
    ComputeFuture: Future<Output = Result<Value>>,
{
    let path = path_for_request(component, request)?;

    if let Some(value) = read_json(&path)? {
        return Ok(CacheResult {
            value,
            cache_hit: true,
        });
    }

    let value = compute().await?;
    write_json(&path, &value)?;

    Ok(CacheResult {
        value,
        cache_hit: false,
    })
}

pub fn path_for_request<Request: Serialize>(component: &str, request: &Request) -> Result<PathBuf> {
    let request_json = serde_json::to_vec(request).context("failed to serialize cache request")?;

    let request_hash = keccak256(&request_json);

    Ok(PathBuf::from(CACHE_ROOT)
        .join(component)
        .join(format!("{request_hash}.json")))
}

// DeserializeOwned означает, что тип можно полностью создать из JSON без ссылок на исходную строку
pub fn read_json<Value: DeserializeOwned>(path: &Path) -> Result<Option<Value>> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let value = serde_json::from_str(&contents) // получаем исходную Rust структуру из JSON 
                  .with_context(|| format!("failed to decode cache file {}", path.display()))?;

            Ok(Some(value))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("failed to read cache file {}", path.display()))
        }
    }
}

pub fn write_json<Value: Serialize>(path: &Path, value: &Value) -> Result<()> {
    let contents = serde_json::to_vec_pretty(value).context("failed to encode cache value")?;
    write_bytes(path, &contents)
}

pub fn write_bytes(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("cache path must have a parent directory")?;

    fs::create_dir_all(parent) // создаем папку для кеша вместе со всеми отсутствующими папками
        .with_context(|| format!("failed to create cache directory {}", parent.display()))?;

    let temporary_path = path.with_extension("json.tmp");

    fs::write(&temporary_path, contents).with_context(|| {
        format!(
            "failed to write temporary cache file {}",
            temporary_path.display()
        )
    })?;

    fs::rename(&temporary_path, path)
        .with_context(|| format!("failed to finalize cache file {}", path.display()))
}
