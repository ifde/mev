use std::{
      fs,
      path::{Path, PathBuf},
  };

  use alloy::primitives::keccak256;
  use anyhow::{Context, Result};
  use serde::{de::DeserializeOwned, Serialize};

  const CACHE_ROOT: &str = "cache";

  pub fn path_for_request<Request: Serialize>(
      component: &str,
      request: &Request,
  ) -> Result<PathBuf> {
      let request_json =
          serde_json::to_vec(request).context("failed to serialize cache request")?;

      let request_hash = keccak256(&request_json);

      Ok(PathBuf::from(CACHE_ROOT)
          .join(component)
          .join(format!("{request_hash}.json")))
  }

  // DeserializeOwned означает, что тип можно полностью создать из JSON без ссылок на исходную строку 
  pub fn read_json<Value: DeserializeOwned>(
      path: &Path,
  ) -> Result<Option<Value>> {
      match fs::read_to_string(path) {
          Ok(contents) => {
              let value = serde_json::from_str(&contents) // получаем исходную Rust структуру из JSON 
                  .with_context(|| format!("failed to decode cache file {}", path.display()))?;

              Ok(Some(value))
          }
          Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
              Ok(None)
          }
          Err(error) => {
              Err(error)
                  .with_context(|| format!("failed to read cache file {}", path.display()))
          }
      }
  }

  pub fn write_json<Value: Serialize>(
      path: &Path,
      value: &Value,
  ) -> Result<()> {
      let parent = path
          .parent()
          .context("cache path must have a parent directory")?;

      fs::create_dir_all(parent) // создаем папку для кеша вместе со всеми отсутствующими папками 
          .with_context(|| format!("failed to create cache directory {}", parent.display()))?;

      let contents =
          serde_json::to_string_pretty(value).context("failed to encode cache value")?;

      fs::write(path, contents)
          .with_context(|| format!("failed to write cache file {}", path.display()))
  }
