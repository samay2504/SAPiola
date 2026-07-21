use crate::mapping::Catalog;
use super::cypher_parser::{CypherAst, Condition};

#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub tenant_id: String,
    pub allowed_labels: Vec<String>, 
}

impl SecurityContext {
    pub fn can_access(&self, label: &str) -> bool {
        self.allowed_labels.is_empty() || self.allowed_labels.contains(&label.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct NodeScan {
    pub var_name: String,
    pub label: String,
    pub table: String,
    pub id_col: String,
}

#[derive(Debug, Clone)]
pub struct EdgeJoin {
    pub edge_var: Option<String>,
    pub edge_label: String,
    pub edge_table: String,
    
    pub source_node_var: String,
    pub source_table: String,
    pub source_id_col: String,
    pub edge_source_col: String,

    pub target_node_var: String,
    pub target_label: String,
    pub target_table: String,
    pub target_id_col: String,
    pub edge_target_col: String,
}

#[derive(Debug, Clone)]
pub struct LogicalPlan {
    pub start_scan: NodeScan,
    pub hops: Vec<EdgeJoin>,
    pub conditions: Vec<Condition>,
    pub returns: Vec<String>,
    pub limit: Option<usize>,
}

impl LogicalPlan {
    pub fn build(ast: CypherAst, catalog: &Catalog, ctx: &SecurityContext) -> anyhow::Result<Self> {
        let start = &ast.path.start_node;
        
        if !ctx.can_access(&start.label) {
            anyhow::bail!("RBAC Error: unauthorized access to node label '{}'", start.label);
        }

        let start_mapping = catalog.nodes.get(&start.label)
            .ok_or_else(|| anyhow::anyhow!("Unknown node label '{}'", start.label))?;

        let start_scan = NodeScan {
            var_name: start.var_name.clone(),
            label: start.label.clone(),
            table: start_mapping.table.clone(),
            id_col: start_mapping.id_column.clone(),
        };

        let mut hops = Vec::new();
        let mut current_var = start.var_name.clone();
        let mut current_mapping = start_mapping.clone();

        for (edge, target) in ast.path.hops {
            if !ctx.can_access(&edge.label) {
                anyhow::bail!("RBAC Error: unauthorized access to edge label '{}'", edge.label);
            }
            if !ctx.can_access(&target.label) {
                anyhow::bail!("RBAC Error: unauthorized access to node label '{}'", target.label);
            }

            let edge_mapping = catalog.edges.get(&edge.label)
                .ok_or_else(|| anyhow::anyhow!("Unknown edge label '{}'", edge.label))?;
            let target_mapping = catalog.nodes.get(&target.label)
                .ok_or_else(|| anyhow::anyhow!("Unknown node label '{}'", target.label))?;

            hops.push(EdgeJoin {
                edge_var: edge.var_name.clone(),
                edge_label: edge.label.clone(),
                edge_table: edge_mapping.table.clone(),

                source_node_var: current_var.clone(),
                source_table: current_mapping.table.clone(),
                source_id_col: current_mapping.id_column.clone(),
                edge_source_col: edge_mapping.source_column.clone(),

                target_node_var: target.var_name.clone(),
                target_label: target.label.clone(),
                target_table: target_mapping.table.clone(),
                target_id_col: target_mapping.id_column.clone(),
                edge_target_col: edge_mapping.target_column.clone(),
            });

            current_var = target.var_name.clone();
            current_mapping = target_mapping.clone();
        }

        Ok(LogicalPlan {
            start_scan,
            hops,
            conditions: ast.conditions,
            returns: ast.returns,
            limit: ast.limit,
        })
    }
}
