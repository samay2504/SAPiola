use async_trait::async_trait;
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;
use serde_json::{Value, Map};
use super::backend::RelationalBackend;

pub struct PostgresBackend {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl PostgresBackend {
    pub async fn new(connection_string: &str) -> anyhow::Result<Self> {
        let manager = PostgresConnectionManager::new_from_stringlike(connection_string, NoTls)?;
        let pool = Pool::builder().build(manager).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl RelationalBackend for PostgresBackend {
    async fn execute(&self, query: &str, params: &[&str]) -> anyhow::Result<Vec<Value>> {
        let conn = self.pool.get().await?;
        
        let stmt = conn.prepare(query).await?;
        
        let mut pg_params = Vec::new();
        for p in params {
            pg_params.push(p as &(dyn tokio_postgres::types::ToSql + Sync));
        }

        let rows = conn.query(&stmt, &pg_params).await?;
        
        let mut results = Vec::new();
        for row in rows {
            let mut obj = Map::new();
            for col in row.columns() {
                let name = col.name().to_string();
                if let Ok(val) = row.try_get::<_, String>(col.name()) {
                    obj.insert(name, Value::String(val));
                } else if let Ok(val) = row.try_get::<_, i32>(col.name()) {
                    obj.insert(name, Value::Number(val.into()));
                } else if let Ok(val) = row.try_get::<_, i64>(col.name()) {
                    obj.insert(name, Value::Number(val.into()));
                } else {
                    obj.insert(name, Value::Null);
                }
            }
            results.push(Value::Object(obj));
        }
        
        Ok(results)
    }
}
