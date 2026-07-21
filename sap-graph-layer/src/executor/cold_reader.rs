use super::backend::RelationalBackend;
use crate::planner::LogicalPlan;
use serde_json::Value;

pub struct ColdReader {
    backend: Box<dyn RelationalBackend>,
}

impl ColdReader {
    pub fn new(backend: Box<dyn RelationalBackend>) -> Self {
        Self { backend }
    }

    /// Compiles a LogicalPlan into a single SQL statement with chained JOINs
    /// and executes it against the RelationalBackend.
    pub async fn execute(&self, plan: LogicalPlan) -> anyhow::Result<Vec<Value>> {
        let (sql, params) = self.compile_to_sql(&plan)?;
        
        let timeout_duration = std::time::Duration::from_secs(30);
        let result = tokio::time::timeout(timeout_duration, self.backend.execute(&sql, &params)).await;
        
        match result {
            Ok(res) => res,
            Err(_) => anyhow::bail!("Query execution timed out after {} seconds (SQL: {})", timeout_duration.as_secs(), sql),
        }
    }

    fn compile_to_sql<'a>(&self, plan: &'a LogicalPlan) -> anyhow::Result<(String, Vec<&'a str>)> {
        let mut sql = String::new();
        let mut params = Vec::new();
        
        sql.push_str("SELECT ");
        if plan.returns.is_empty() {
            sql.push_str("*");
        } else {
            let mut selects = Vec::new();
            for ret in &plan.returns {
                selects.push(format!("{}.*", ret));
            }
            sql.push_str(&selects.join(", "));
        }
        
        sql.push_str(&format!(" FROM {} {}", plan.start_scan.table, plan.start_scan.var_name));

        for hop in &plan.hops {
            if let Some(edge_var) = &hop.edge_var {
                sql.push_str(&format!(" INNER JOIN {} {} ON {}.{} = {}.{}", 
                    hop.edge_table, edge_var, 
                    hop.source_node_var, hop.source_id_col, 
                    edge_var, hop.edge_source_col));
                    
                sql.push_str(&format!(" INNER JOIN {} {} ON {}.{} = {}.{}", 
                    hop.target_table, hop.target_node_var, 
                    edge_var, hop.edge_target_col, 
                    hop.target_node_var, hop.target_id_col));
            } else {
                let temp_edge_var = format!("{}_edge", hop.target_node_var);
                sql.push_str(&format!(" INNER JOIN {} {} ON {}.{} = {}.{}", 
                    hop.edge_table, temp_edge_var, 
                    hop.source_node_var, hop.source_id_col, 
                    temp_edge_var, hop.edge_source_col));
                    
                sql.push_str(&format!(" INNER JOIN {} {} ON {}.{} = {}.{}", 
                    hop.target_table, hop.target_node_var, 
                    temp_edge_var, hop.edge_target_col, 
                    hop.target_node_var, hop.target_id_col));
            }
        }

        if !plan.conditions.is_empty() {
            sql.push_str(" WHERE ");
            let mut cond_strs = Vec::new();
            
            for (i, cond) in plan.conditions.iter().enumerate() {
                cond_strs.push(format!("{}.{} = ${}", cond.var_name, cond.prop_name, i + 1));
                params.push(cond.value.as_str());
            }
            sql.push_str(&cond_strs.join(" AND "));
        }

        if let Some(limit) = plan.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        Ok((sql, params))
    }
}
