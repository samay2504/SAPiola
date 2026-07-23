use async_trait::async_trait;
use hdbconnect_async::{ConnectParams, Connection, HdbValue};
use serde_json::{Map, Number, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use super::backend::RelationalBackend;

pub struct HanaBackend {
    conn: Arc<Mutex<Connection>>,
}

impl HanaBackend {
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let mut builder = ConnectParams::builder();
        let params = builder
            .hostname(url)
            .port(443)
            .tls_without_server_verification()
            .build()?;
        let conn = Connection::new(params).await?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn from_credentials(
        host: &str,
        port: u16,
        user: &str,
        pass: &str,
    ) -> anyhow::Result<Self> {
        let mut builder = ConnectParams::builder();
        builder
            .hostname(host)
            .port(port)
            .dbuser(user)
            .password(pass)
            .tls_without_server_verification();
        let params = builder.build()?;
        let conn = Connection::new(params).await?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl RelationalBackend for HanaBackend {
    async fn execute(&self, query: &str, _params: &[&str]) -> anyhow::Result<Vec<Value>> {
        let conn = self.conn.lock().await;
        let response = match conn.statement(query).await {
            Ok(resp) => resp,
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("259") || err_msg.contains("invalid table name") {
                    return Ok(vec![]);
                }
                return Err(e.into());
            }
        };

        let result_set = match response.into_result_set() {
            Ok(rs) => rs,
            Err(_) => return Ok(vec![]),
        };
        let rows = result_set.into_rows().await?;
        let mut results = Vec::new();
        for row in rows {
            let mut obj = Map::new();
            for (idx, field) in row.into_iter().enumerate() {
                let col_name = format!("col_{}", idx);
                let json_val = hdb_value_to_json(field);
                obj.insert(col_name, json_val);
            }
            results.push(Value::Object(obj));
        }

        Ok(results)
    }
}


fn hdb_value_to_json(val: HdbValue) -> Value {
    match val {
        HdbValue::NULL => Value::Null,
        HdbValue::BOOLEAN(b) => Value::Bool(b),
        HdbValue::TINYINT(i) => Value::Number(Number::from(i)),
        HdbValue::SMALLINT(i) => Value::Number(Number::from(i)),
        HdbValue::INT(i) => Value::Number(Number::from(i)),
        HdbValue::BIGINT(i) => Value::Number(Number::from(i)),
        HdbValue::DOUBLE(f) => Number::from_f64(f).map_or(Value::Null, Value::Number),
        HdbValue::DECIMAL(dec) => Value::String(dec.to_string()),
        HdbValue::STRING(s) => Value::String(s),
        other => Value::String(format!("{:?}", other)),
    }
}

