use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait RelationalBackend: Send + Sync {
    /// Executes a parameterized query, returning results as JSON objects.
    async fn execute(&self, query: &str, params: &[&str]) -> anyhow::Result<Vec<Value>>;
}
