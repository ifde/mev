// ==============================
// Общий GraphQL-клиент Subgraph API.
// ==============================

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

pub struct SubgraphClient {
    http: Client,
    url: String,
    api_key: String,
}

impl SubgraphClient {
    pub fn new(url: String, api_key: String) -> Self {
        Self {
            http: Client::new(),
            url,
            api_key,
        }
    }

    // конкретный GraphQL-запрос
    pub async fn query<Data: DeserializeOwned>(
        &self,
        query: &str,
        variables: Value,
    ) -> Result<Data> {
        // делаем API запрос к SubGraph API
        let response = self
            .http
            .post(&self.url)
            .bearer_auth(&self.api_key)
            .json(&GraphQlRequest { query, variables })
            .send()
            .await
            .context("subgraph HTTP request failed")?
            .error_for_status()
            .context("subgraph returned non-success HTTP status")?
            // преобразуем JSON в типы Rust, включая все вложенные поля
            .json::<GraphQlResponse<Data>>()
            .await
            .context("failed to decode subgraph response")?;

        if let Some(errors) = response.errors {
            let messages = errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join(", ");

            bail!("subgraph returned errors: {messages}");
        }

        response.data.context("subgraph response has no data")
    }
}

#[derive(Serialize)]
struct GraphQlRequest<'a> {
    query: &'a str,
    variables: Value, // передаем параметры через JSON
}

// ответ
// везде в ответах добавляем трейт Deserialize
#[derive(Deserialize)]
struct GraphQlResponse<Data> {
    data: Option<Data>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
}
