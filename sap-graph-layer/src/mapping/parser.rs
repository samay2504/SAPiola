// Mapping DSL parser using nom.
//
// Grammar (EBNF):
//   dsl       = { statement NEWLINE }
//   statement = node_stmt | edge_stmt | comment
//   node_stmt = "NODE" label "FROM" table ["JOIN" table] "WITH" "id" "=" col
//               [ "PROPERTIES" "(" col {"," col} ")" ]
//   edge_stmt = "EDGE" label "FROM" table ["JOIN" table]
//               "USING" col "->" col
//               [ "PROPERTIES" "(" col {"," col} ")" ]
//   comment   = "#" <rest of line>
//
// Big-O: O(n) in input bytes.
// Allocation: one NodeMapping or EdgeMapping per statement, sized to the number
// of property columns. No intermediate allocations.

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while1},
    character::complete::{char, multispace0, multispace1, not_line_ending},
    combinator::{map, opt, value},
    multi::separated_list1,
    sequence::{delimited, preceded, tuple},
    IResult,
};

use super::types::{Catalog, EdgeMapping, NodeMapping, TableRef};

// ─── Primitives ─────────────────────────────────────────────────────────────

fn ident(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
}

fn ws(input: &str) -> IResult<&str, &str> {
    multispace0(input)
}

fn ws1(input: &str) -> IResult<&str, &str> {
    multispace1(input)
}

// ─── Table reference ────────────────────────────────────────────────────────

fn table_ref(input: &str) -> IResult<&str, TableRef> {
    let (input, table) = ident(input)?;
    // Optional JOIN <table>
    let (input, join_table) = opt(preceded(
        tuple((ws1, tag_no_case("JOIN"), ws1)),
        map(ident, str::to_string),
    ))(input)?;
    Ok((input, TableRef { table: table.to_string(), join_table }))
}

// ─── Property list ──────────────────────────────────────────────────────────

fn prop_list(input: &str) -> IResult<&str, Vec<String>> {
    let (input, _) = tuple((ws1, tag_no_case("PROPERTIES"), ws, char('(')))(input)?;
    let (input, cols) = separated_list1(
        delimited(ws, char(','), ws),
        map(ident, str::to_string),
    )(input)?;
    let (input, _) = tuple((ws, char(')')))(input)?;
    Ok((input, cols))
}

// ─── NODE statement ─────────────────────────────────────────────────────────

fn node_stmt(input: &str) -> IResult<&str, NodeMapping> {
    let (input, _) = tuple((ws, tag_no_case("NODE"), ws1))(input)?;
    let (input, label) = map(ident, str::to_string)(input)?;
    let (input, _) = tuple((ws1, tag_no_case("FROM"), ws1))(input)?;
    let (input, source) = table_ref(input)?;
    let (input, _) = tuple((ws1, tag_no_case("WITH"), ws1, tag_no_case("id"), ws, char('='), ws))(input)?;
    let (input, id_column) = map(ident, str::to_string)(input)?;
    let (input, properties) = opt(prop_list)(input)?;
    Ok((
        input,
        NodeMapping {
            label,
            source,
            id_column,
            properties: properties.unwrap_or_default(),
        },
    ))
}

// ─── EDGE statement ─────────────────────────────────────────────────────────

fn edge_stmt(input: &str) -> IResult<&str, EdgeMapping> {
    let (input, _) = tuple((ws, tag_no_case("EDGE"), ws1))(input)?;
    let (input, label) = map(ident, str::to_string)(input)?;
    let (input, _) = tuple((ws1, tag_no_case("FROM"), ws1))(input)?;
    let (input, source) = table_ref(input)?;
    let (input, _) = tuple((ws1, tag_no_case("USING"), ws1))(input)?;
    let (input, from_column) = map(ident, str::to_string)(input)?;
    let (input, _) = tuple((ws, tag("->"), ws))(input)?;
    let (input, to_column) = map(ident, str::to_string)(input)?;
    let (input, properties) = opt(prop_list)(input)?;
    Ok((
        input,
        EdgeMapping {
            label,
            source,
            from_column,
            to_column,
            properties: properties.unwrap_or_default(),
        },
    ))
}

// ─── Comment ────────────────────────────────────────────────────────────────

fn comment(input: &str) -> IResult<&str, ()> {
    value((), tuple((ws, char('#'), not_line_ending)))(input)
}

// ─── Full DSL document ──────────────────────────────────────────────────────

enum Statement {
    Node(NodeMapping),
    Edge(EdgeMapping),
    Comment,
}

fn statement(input: &str) -> IResult<&str, Statement> {
    alt((
        map(node_stmt, Statement::Node),
        map(edge_stmt, Statement::Edge),
        map(comment, |_| Statement::Comment),
    ))(input)
}

/// Parse a complete DSL document into a Catalog.
///
/// Returns an error string if any statement fails to parse.
/// Blank lines and `#` comments are silently skipped.
pub fn parse_catalog(input: &str) -> Result<Catalog, String> {
    let mut catalog = Catalog::default();

    for (line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        match statement(line) {
            Ok((_, Statement::Node(n))) => {
                catalog.nodes.insert(n.label.clone(), n);
            }
            Ok((_, Statement::Edge(e))) => {
                catalog.edges.insert(e.label.clone(), e);
            }
            Ok((_, Statement::Comment)) => {}
            Err(e) => {
                return Err(format!("Parse error on line {}: {:?}", line_no + 1, e));
            }
        }
    }

    Ok(catalog)
}
