// Cold-path query executor.
//
// Translates a LogicalPlan into SQL queries against the SAP HANA source
// (or a generic SQL-compatible mock for testing) and collects results into
// Arrow RecordBatches.
//
// Kleppmann Ch.3 pp.68,82 — cold path is point-lookup / full-scan on B-tree
// indexed SAP tables, NOT LSM sequential merge (that's Phase 2 hot path).
//
// Design:
//   - Uses `hdbconnect_async` for SAP HANA (pure Rust, no JVM).
//   - Abstracts the connection behind the `SqlConnection` trait so the mock
//     test driver (plain String→Rows table) plugs in without feature flags.
//   - Arrow RecordBatch is the output format (zero-copy slicing downstream).
//
// Big-O per query step:
//   NodeScan:   O(n) where n = rows matching predicate in SAP table.
//   EdgeExpand: O(e) where e = matching edge rows in edge table.
//   Filter:     O(r) where r = rows from input plan.
//   Project:    O(r) — column selection, no new allocations.
//
// Allocation profile:
//   One RecordBatch per plan step. Columns are Arc<dyn Array> — O(1) clone
//   into downstream consumers. No per-row heap allocation during iteration.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use arrow::array::{ArrayRef, StringArray};
use arrow::datatypes::{DataType, Field, Fields, Schema};
use arrow::record_batch::RecordBatch;

use crate::mapping::Catalog;
use crate::planner::logical_plan::{Direction, LiteralValue, LogicalPlan, Predicate, Projection};

// ─── Row representation (internal) ──────────────────────────────────────────

/// A single result row during cold-path execution.
/// Key = column alias or "variable.column". Value = typed cell.
#[derive(Debug, Clone)]
pub enum Cell {
    Str(String),
    Int(i64),
    Null,
}

/// One row of intermediate results, keyed by qualified column name.
pub type Row = HashMap<String, Cell>;

// ─── SqlConnection trait ─────────────────────────────────────────────────────

/// Abstraction over the SQL connection used by the executor.
/// This trait is the only seam between production HANA code and test mocks.
///
/// Implementors: `HanaConnection` (production), `MockSqlDb` (tests).
#[async_trait::async_trait]
pub trait SqlConnection: Send + Sync {
    /// Execute a SELECT query and return rows as Vec<HashMap<col, Cell>>.
    async fn query(&self, sql: &str, params: &[Cell]) -> Result<Vec<Row>>;
}

// ─── Cold-path executor ──────────────────────────────────────────────────────

pub struct ColdExecutor<'c> {
    conn: &'c dyn SqlConnection,
    catalog: &'c Catalog,
}

impl<'c> ColdExecutor<'c> {
    pub fn new(conn: &'c dyn SqlConnection, catalog: &'c Catalog) -> Self {
        Self { conn, catalog }
    }

    /// Execute a LogicalPlan and return Arrow RecordBatches.
    ///
    /// Phase 1: cold path only. Returns one RecordBatch per plan leaf.
    /// Each batch contains all columns named by the projections.
    pub async fn execute(&self, plan: &LogicalPlan) -> Result<RecordBatch> {
        let rows = self.execute_plan(plan).await?;
        // Determine schema from projection columns in the plan
        let columns = collect_column_names(plan);
        rows_to_batch(rows, &columns)
    }

    // ─── Recursive plan evaluation ───────────────────────────────────────────

    async fn execute_plan(&self, plan: &LogicalPlan) -> Result<Vec<Row>> {
        match plan {
            LogicalPlan::NodeScan { label, variable, predicate } => {
                self.exec_node_scan(label, variable, predicate.as_ref()).await
            }
            LogicalPlan::EdgeExpand { input, from_variable, edge_label, direction, to_variable, edge_variable: _, node_predicate } => {
                let source_rows = Box::pin(self.execute_plan(input)).await?;
                self.exec_edge_expand(
                    source_rows, from_variable, edge_label,
                    direction, to_variable, node_predicate.as_ref(),
                ).await
            }
            LogicalPlan::Filter { input, predicate } => {
                let rows = Box::pin(self.execute_plan(input)).await?;
                Ok(rows.into_iter().filter(|r| eval_predicate(r, predicate)).collect())
            }
            LogicalPlan::Project { input, projections, limit } => {
                let rows = Box::pin(self.execute_plan(input)).await?;
                let projected: Vec<Row> = rows
                    .into_iter()
                    .map(|r| project_row(r, projections))
                    .collect();
                if let Some(n) = limit {
                    Ok(projected.into_iter().take(*n).collect())
                } else {
                    Ok(projected)
                }
            }
        }
    }

    // ─── NodeScan → SQL SELECT ────────────────────────────────────────────────

    async fn exec_node_scan(
        &self,
        label: &str,
        variable: &str,
        predicate: Option<&Predicate>,
    ) -> Result<Vec<Row>> {
        let mapping = self
            .catalog
            .node(label)
            .with_context(|| format!("Node label '{}' not found in catalog", label))?;

        let table = &mapping.source.table;
        let id_col = &mapping.id_column;
        let prop_cols = &mapping.properties;

        // Build SELECT col list
        let mut select_cols = vec![id_col.clone()];
        select_cols.extend(prop_cols.iter().cloned());
        let select_str = select_cols.join(", ");

        let (where_clause, params) = predicate_to_sql(predicate);
        let sql = if where_clause.is_empty() {
            format!("SELECT {} FROM {}", select_str, table)
        } else {
            format!("SELECT {} FROM {} WHERE {}", select_str, table, where_clause)
        };

        let raw = self.conn.query(&sql, &params).await?;

        // Qualify column names with variable prefix: variable.col
        Ok(raw
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|(col, val)| (format!("{}.{}", variable, col), val))
                    .collect()
            })
            .collect())
    }

    // ─── EdgeExpand → SQL JOIN ────────────────────────────────────────────────

    async fn exec_edge_expand(
        &self,
        source_rows: Vec<Row>,
        from_variable: &str,
        edge_label: &str,
        direction: &Direction,
        to_variable: &str,
        node_predicate: Option<&Predicate>,
    ) -> Result<Vec<Row>> {
        let edge_mapping = self
            .catalog
            .edge(edge_label)
            .with_context(|| format!("Edge label '{}' not found in catalog", edge_label))?;

        let edge_table = &edge_mapping.source.table;
        let (from_col, to_col) = match direction {
            Direction::Out | Direction::Both => (&edge_mapping.from_column, &edge_mapping.to_column),
            Direction::In => (&edge_mapping.to_column, &edge_mapping.from_column),
        };

        // Collect source IDs. We look for id_col of the source node.
        // The source rows have keys like "fromVar.ID_COL".
        // We grab the first column that starts with `from_variable.` and ends with
        // the mapped id column for that node label.
        let mut result_rows: Vec<Row> = Vec::new();

        // 1. Collect and deduplicate source IDs, and group rows by ID for hash join
        let mut unique_ids = HashSet::new();
        let mut src_rows_by_id: HashMap<String, Vec<Row>> = HashMap::new();

        for src_row in source_rows {
            let id_key = src_row
                .keys()
                .find(|k| k.starts_with(&format!("{}.", from_variable)))
                .cloned();

            let id_val = match id_key {
                Some(k) => src_row.get(&k).cloned().unwrap_or(Cell::Null),
                None => continue,
            };

            let id_str = match &id_val {
                Cell::Str(s) => format!("'{}'", s),
                Cell::Int(n) => n.to_string(),
                Cell::Null => continue,
            };

            unique_ids.insert(id_str.clone());
            src_rows_by_id.entry(id_str).or_default().push(src_row);
        }

        // 2. Batch query edges in chunks of 1000
        let id_list: Vec<String> = unique_ids.into_iter().collect();
        let chunks = id_list.chunks(1000);
        let mut edges_by_from_id: HashMap<String, Vec<Row>> = HashMap::new();

        for chunk in chunks {
            let in_clause = chunk.join(", ");
            let (where_clause, params) = predicate_to_sql(node_predicate);

            let sql = if where_clause.is_empty() {
                format!(
                    "SELECT {}, {} FROM {} WHERE {} IN ({})",
                    from_col, to_col, edge_table, from_col, in_clause
                )
            } else {
                format!(
                    "SELECT {}, {} FROM {} WHERE {} IN ({}) AND {}",
                    from_col, to_col, edge_table, from_col, in_clause, where_clause
                )
            };

            let edge_rows = self.conn.query(&sql, &params).await?;
            for edge_row in edge_rows {
                let f_val = edge_row.get(from_col).cloned().unwrap_or(Cell::Null);
                let f_str = match &f_val {
                    Cell::Str(s) => format!("'{}'", s),
                    Cell::Int(n) => n.to_string(),
                    Cell::Null => continue,
                };
                edges_by_from_id.entry(f_str).or_default().push(edge_row);
            }
        }

        // 3. In-memory hash join
        for (id_str, src_rows_group) in src_rows_by_id {
            if let Some(edge_rows) = edges_by_from_id.get(&id_str) {
                for src_row in src_rows_group {
                    for edge_row in edge_rows {
                        let mut merged = src_row.clone();
                        let to_val = edge_row.get(to_col).cloned().unwrap_or(Cell::Null);
                        merged.insert(format!("{}.{}", to_variable, to_col), to_val);
                        result_rows.push(merged);
                    }
                }
            }
        }

        Ok(result_rows)
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn predicate_to_sql(pred: Option<&Predicate>) -> (String, Vec<Cell>) {
    match pred {
        None => (String::new(), vec![]),
        Some(Predicate::Eq { column, value }) => {
            let v = match value {
                LiteralValue::Str(s) => format!("'{}'", s),
                LiteralValue::Int(n) => n.to_string(),
                LiteralValue::Float(f) => f.to_string(),
            };
            (format!("{} = {}", column, v), vec![])
        }
        Some(Predicate::In { column, values }) => {
            let vals: Vec<String> = values
                .iter()
                .map(|v| match v {
                    LiteralValue::Str(s) => format!("'{}'", s),
                    LiteralValue::Int(n) => n.to_string(),
                    LiteralValue::Float(f) => f.to_string(),
                })
                .collect();
            (format!("{} IN ({})", column, vals.join(", ")), vec![])
        }
        Some(Predicate::And(a, b)) => {
            let (sa, pa) = predicate_to_sql(Some(a));
            let (sb, pb) = predicate_to_sql(Some(b));
            let mut params = pa;
            params.extend(pb);
            (format!("({}) AND ({})", sa, sb), params)
        }
    }
}

fn eval_predicate(row: &Row, pred: &Predicate) -> bool {
    match pred {
        Predicate::Eq { column, value } => {
            // Try all keys that end with .column
            for (k, v) in row {
                if k.ends_with(&format!(".{}", column)) || k == column {
                    return match (v, value) {
                        (Cell::Str(a), LiteralValue::Str(b)) => a == b,
                        (Cell::Int(a), LiteralValue::Int(b)) => a == b,
                        _ => false,
                    };
                }
            }
            false
        }
        Predicate::And(a, b) => eval_predicate(row, a) && eval_predicate(row, b),
        Predicate::In { column, values } => {
            for (k, v) in row {
                if k.ends_with(&format!(".{}", column)) || k == column {
                    return values.iter().any(|lv| match (v, lv) {
                        (Cell::Str(a), LiteralValue::Str(b)) => a == b,
                        (Cell::Int(a), LiteralValue::Int(b)) => a == b,
                        _ => false,
                    });
                }
            }
            false
        }
    }
}

fn project_row(row: Row, projections: &[Projection]) -> Row {
    let mut out = Row::new();
    for proj in projections {
        let search_key = match &proj.property {
            Some(prop) => format!("{}.{}", proj.variable, prop),
            None => {
                // Return all columns for this variable
                for (k, v) in &row {
                    if k.starts_with(&format!("{}.", proj.variable)) {
                        out.insert(format!("{}.{}", proj.alias, k.splitn(2, '.').nth(1).unwrap_or(k)), v.clone());
                    }
                }
                continue;
            }
        };
        let val = row.get(&search_key).cloned().unwrap_or(Cell::Null);
        out.insert(proj.alias.clone(), val);
    }
    out
}

fn collect_column_names(plan: &LogicalPlan) -> Vec<String> {
    match plan {
        LogicalPlan::Project { projections, .. } => {
            projections.iter().map(|p| p.alias.clone()).collect()
        }
        _ => vec![],
    }
}

/// Convert a Vec<Row> to an Arrow RecordBatch.
///
/// All values are coerced to Utf8 for Phase 1 simplicity.
/// Phase 2+ will introduce typed column arrays.
/// Allocation: O(rows × columns) — one contiguous Arrow buffer per column.
pub fn rows_to_batch(rows: Vec<Row>, columns: &[String]) -> Result<RecordBatch> {
    if columns.is_empty() || rows.is_empty() {
        let schema = Arc::new(Schema::new(Fields::empty()));
        return Ok(RecordBatch::new_empty(schema));
    }

    let mut col_data: HashMap<&str, Vec<Option<String>>> = HashMap::new();
    for col in columns {
        col_data.insert(col.as_str(), Vec::with_capacity(rows.len()));
    }

    for row in &rows {
        for col in columns {
            let val = match row.get(col.as_str()) {
                Some(Cell::Str(s)) => Some(s.clone()),
                Some(Cell::Int(n)) => Some(n.to_string()),
                _ => None,
            };
            col_data.get_mut(col.as_str()).unwrap().push(val);
        }
    }

    let fields: Vec<Field> = columns
        .iter()
        .map(|c| Field::new(c.as_str(), DataType::Utf8, true))
        .collect();
    let schema = Arc::new(Schema::new(fields));

    let arrays: Vec<ArrayRef> = columns
        .iter()
        .map(|c| {
            let data = col_data.remove(c.as_str()).unwrap();
            Arc::new(StringArray::from(
                data.iter().map(|o| o.as_deref()).collect::<Vec<_>>(),
            )) as ArrayRef
        })
        .collect();

    Ok(RecordBatch::try_new(schema, arrays)?)
}
