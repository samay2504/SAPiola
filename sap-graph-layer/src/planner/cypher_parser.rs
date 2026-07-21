use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "planner/cypher.pest"]
pub struct CypherParser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePattern {
    pub var_name: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgePattern {
    pub var_name: Option<String>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPattern {
    pub start_node: NodePattern,
    pub hops: Vec<(EdgePattern, NodePattern)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    pub var_name: String,
    pub prop_name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CypherAst {
    pub path: PathPattern,
    pub conditions: Vec<Condition>,
    pub returns: Vec<String>,
    pub limit: Option<usize>,
}

impl CypherAst {
    pub fn parse(query: &str) -> anyhow::Result<Self> {
        let mut parsed = CypherParser::parse(Rule::query, query)?;
        let query_rule = parsed.next().unwrap();

        let mut path = None;
        let mut conditions = Vec::new();
        let mut returns = Vec::new();
        let mut limit = None;

        for clause in query_rule.into_inner() {
            match clause.as_rule() {
                Rule::match_clause => {
                    let path_rule = clause.into_inner().next().unwrap();
                    let mut path_inner = path_rule.into_inner();
                    
                    let start_node_rule = path_inner.next().unwrap();
                    let mut sn_inner = start_node_rule.into_inner();
                    let start_node = NodePattern {
                        var_name: sn_inner.next().unwrap().as_str().to_string(),
                        label: sn_inner.next().unwrap().as_str().to_string(),
                    };

                    let mut hops = Vec::new();
                    while let Some(edge_rule) = path_inner.next() {
                        let node_rule = path_inner.next().unwrap();
                        
                        let mut var_name = None;
                        let mut label = String::new();
                        for inner in edge_rule.into_inner() {
                            if inner.as_rule() == Rule::var_name {
                                var_name = Some(inner.as_str().to_string());
                            } else if inner.as_rule() == Rule::label {
                                label = inner.as_str().to_string();
                            }
                        }
                        
                        let edge_pattern = EdgePattern { var_name, label };

                        let mut nr_inner = node_rule.into_inner();
                        let node_pattern = NodePattern {
                            var_name: nr_inner.next().unwrap().as_str().to_string(),
                            label: nr_inner.next().unwrap().as_str().to_string(),
                        };
                        hops.push((edge_pattern, node_pattern));
                    }
                    path = Some(PathPattern { start_node, hops });
                }
                Rule::where_clause => {
                    for cond_rule in clause.into_inner() {
                        let mut c_inner = cond_rule.into_inner();
                        let var_name = c_inner.next().unwrap().as_str().to_string();
                        let prop_name = c_inner.next().unwrap().as_str().to_string();
                        let value = c_inner.next().unwrap().as_str().to_string();
                        let value = if value.starts_with('\'') || value.starts_with('"') {
                            value[1..value.len()-1].to_string()
                        } else {
                            value
                        };
                        conditions.push(Condition { var_name, prop_name, value });
                    }
                }
                Rule::return_clause => {
                    for ret_rule in clause.into_inner() {
                        returns.push(ret_rule.as_str().to_string());
                    }
                }
                Rule::limit_clause => {
                    let num_str = clause.into_inner().next().unwrap().as_str();
                    limit = Some(num_str.parse()?);
                }
                Rule::EOI => {}
                _ => unreachable!(),
            }
        }

        Ok(CypherAst {
            path: path.unwrap(),
            conditions,
            returns,
            limit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_multi_hop() {
        let q = "MATCH (m:Material)-[:SuppliedBy]->(v:Vendor)-[e:LocatedIn]->(c:Country) WHERE m.MAKTX = 'Steel' AND v.LIFNR = 123 RETURN m, v LIMIT 10";
        let ast = CypherAst::parse(q).unwrap();
        
        assert_eq!(ast.path.start_node.label, "Material");
        assert_eq!(ast.path.hops.len(), 2);
        assert_eq!(ast.path.hops[0].0.label, "SuppliedBy");
        assert_eq!(ast.path.hops[1].0.label, "LocatedIn");
        assert_eq!(ast.path.hops[1].0.var_name.as_deref(), Some("e"));
        assert_eq!(ast.conditions.len(), 2);
        assert_eq!(ast.conditions[0].prop_name, "MAKTX");
        assert_eq!(ast.conditions[0].value, "Steel");
        assert_eq!(ast.returns, vec!["m", "v"]);
        assert_eq!(ast.limit, Some(10));
    }
}
