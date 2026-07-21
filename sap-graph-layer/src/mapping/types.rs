// Mapping DSL types.
//
// Kleppmann Ch.2 pp.48-49 — "Property graphs": a node has a label, a set of
// properties, and edges carry their own label + properties. Our mapping DSL
// projects SAP relational tables onto exactly this model without materialising
// a copy of the data.
//
// Big-O / allocation profile:
//   NodeMapping: O(p) where p = number of property column names.
//   EdgeMapping: O(p) similarly.
//   Catalog: HashMap with O(1) amortised lookup.
//   No heap allocation on the hot query path — catalog is read-only after load.

use std::collections::HashMap;

/// A column name in a SAP table.
pub type ColumnName = String;

/// The SAP table a node or edge is sourced from.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRef {
    /// Primary table (e.g. "MARA", "LFA1").
    pub table: String,
    /// Optional join table for edge mappings (e.g. "EKKO" joined to "EKPO").
    pub join_table: Option<String>,
}

/// A logical graph node mapped from one SAP table.
///
/// Example DSL:
///   NODE Material FROM MARA WITH id = MATNR
///     PROPERTIES (MAKTX, MTART, MATKL)
#[derive(Debug, Clone, PartialEq)]
pub struct NodeMapping {
    /// Graph label (e.g. "Material").
    pub label: String,
    /// Source table reference.
    pub source: TableRef,
    /// SAP column used as the stable node ID (maps to dense i64 via SAP_KEY_INDEX).
    pub id_column: ColumnName,
    /// Property columns to expose. Order preserved for schema introspection.
    pub properties: Vec<ColumnName>,
}

/// A directed edge between two node labels, sourced from a SAP table.
///
/// Example DSL:
///   EDGE SuppliedBy FROM EKPO USING MATNR -> LIFNR
///     PROPERTIES (MENGE, NETPR)
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeMapping {
    /// Graph edge label (e.g. "SuppliedBy").
    pub label: String,
    /// Source table (may be a join table).
    pub source: TableRef,
    /// Column in source table that is the FK to the *from* node.
    pub from_column: ColumnName,
    /// Column in source table that is the FK to the *to* node.
    pub to_column: ColumnName,
    /// Property columns on the edge itself.
    pub properties: Vec<ColumnName>,
}

/// In-memory catalog built from the parsed DSL.
///
/// Allocation: two HashMaps sized at construction, never grown after load.
/// Lookup: O(1) amortised per label.
#[derive(Debug, Default, Clone)]
pub struct Catalog {
    pub nodes: HashMap<String, NodeMapping>,
    pub edges: HashMap<String, EdgeMapping>,
}

impl Catalog {
    pub fn node(&self, label: &str) -> Option<&NodeMapping> {
        self.nodes.get(label)
    }

    pub fn edge(&self, label: &str) -> Option<&EdgeMapping> {
        self.edges.get(label)
    }
}
