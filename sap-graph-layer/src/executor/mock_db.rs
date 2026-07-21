// Mock SQL connection for integration tests.
//
// Simulates a SAP-shaped in-memory database (MARA, LFA1, EKPO tables).
// Parses the SQL string from ColdExecutor to return the appropriate rows.
// This avoids a real SAP HANA connection in tests while exercising the full
// plan-to-SQL-to-RecordBatch path.

use std::collections::HashMap;
use anyhow::Result;
use async_trait::async_trait;

use super::cold_executor::{Cell, Row, SqlConnection};

pub struct MockSqlDb {
    /// table_name -> Vec<row> where row = col_name -> Cell
    pub tables: HashMap<String, Vec<HashMap<String, Cell>>>,
}

impl MockSqlDb {
    pub fn new() -> Self {
        Self { tables: HashMap::new() }
    }

    pub fn insert_row(&mut self, table: &str, row: HashMap<&str, Cell>) {
        self.tables
            .entry(table.to_string())
            .or_default()
            .push(row.into_iter().map(|(k, v)| (k.to_string(), v)).collect());
    }
}

enum Filter {
    Eq(String, String),
    In(String, Vec<String>),
}

#[async_trait]
impl SqlConnection for MockSqlDb {
    async fn query(&self, sql: &str, _params: &[Cell]) -> Result<Vec<Row>> {
        // Parse "SELECT col1, col2, ... FROM TABLE WHERE col = 'val'"
        // Minimal SQL parse sufficient for our generated queries only.
        let sql_upper = sql.to_uppercase();

        // Extract table name
        let from_pos = sql_upper.find("FROM ").ok_or_else(|| anyhow::anyhow!("No FROM in SQL"))?;
        let after_from = &sql[from_pos + 5..].trim_start();
        let table_end = after_from
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after_from.len());
        let table_name = after_from[..table_end].to_uppercase();

        let all_rows = match self.tables.get(&table_name) {
            Some(rows) => rows,
            None => return Ok(vec![]),
        };

        // Extract selected columns (between SELECT and FROM)
        let select_part = &sql[7..from_pos].trim();
        let selected_cols: Vec<&str> = select_part.split(',').map(|c| c.trim()).collect();

        // Extract WHERE filters
        let where_filters = parse_where(sql);

        let mut result = Vec::new();
        'rows: for row in all_rows {
            // Apply WHERE filters
            for filter in &where_filters {
                match filter {
                    Filter::Eq(col, val) => {
                        let col_upper = col.to_uppercase();
                        let row_val = row.get(&col_upper).or_else(|| row.get(col.as_str()));
                        match row_val {
                            Some(Cell::Str(s)) if s.to_uppercase() == val.to_uppercase() => {}
                            Some(Cell::Int(n)) if n.to_string() == *val => {}
                            Some(Cell::Str(s)) if s == val => {}
                            _ => continue 'rows,
                        }
                    }
                    Filter::In(col, vals) => {
                        let col_upper = col.to_uppercase();
                        let row_val = row.get(&col_upper).or_else(|| row.get(col.as_str()));
                        match row_val {
                            Some(Cell::Str(s)) => {
                                if !vals.iter().any(|v| s.to_uppercase() == v.to_uppercase() || s == v) {
                                    continue 'rows;
                                }
                            }
                            Some(Cell::Int(n)) => {
                                let n_str = n.to_string();
                                if !vals.iter().any(|v| &n_str == v) {
                                    continue 'rows;
                                }
                            }
                            _ => continue 'rows,
                        }
                    }
                }
            }

            // Project selected columns
            let mut projected = HashMap::new();
            for col in &selected_cols {
                let col_upper = col.to_uppercase();
                let cell = row
                    .get(&col_upper)
                    .or_else(|| row.get(*col))
                    .cloned()
                    .unwrap_or(Cell::Null);
                projected.insert(col_upper, cell);
            }
            result.push(projected);
        }

        Ok(result)
    }
}

/// Parse WHERE clauses of the form "col = 'val'", "col = N", or "col IN ('v1', 'v2')"
fn parse_where(sql: &str) -> Vec<Filter> {
    let mut filters = Vec::new();
    let sql_upper = sql.to_uppercase();
    if let Some(where_pos) = sql_upper.find("WHERE ") {
        let where_clause = &sql[where_pos + 6..];
        for condition in where_clause.split(" AND ") {
            let cond_upper = condition.to_uppercase();
            if let Some(in_pos) = cond_upper.find(" IN ") {
                let col = condition[..in_pos].trim().to_string();
                let after_in = condition[in_pos + 4..].trim();
                let list_str = after_in.trim_start_matches('(').trim_end_matches(')');
                let vals: Vec<String> = list_str
                    .split(',')
                    .map(|s| s.trim().trim_matches('\'').to_string())
                    .collect();
                filters.push(Filter::In(col, vals));
            } else {
                let parts: Vec<&str> = condition.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let col = parts[0].trim().to_string();
                    let val = parts[1].trim().trim_matches('\'').trim().to_string();
                    filters.push(Filter::Eq(col, val));
                }
            }
        }
    }
    filters
}
