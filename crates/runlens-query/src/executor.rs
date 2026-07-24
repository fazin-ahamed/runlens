use rusqlite::Connection;
use serde_json::Value;

use crate::ast::*;
use crate::error::RqlError;

pub fn execute(conn: &Connection, query: &Query) -> Result<Vec<Value>, RqlError> {
    let (sql, params) = to_sql(query)?;
    let mut stmt = conn.prepare(&sql)?;
    let col_names: Vec<String> = (0..stmt.column_count()).map(|i| stmt.column_name(i).unwrap_or("?").to_string()).collect();
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        let mut map = serde_json::Map::new();
        for name in &col_names {
            let val: rusqlite::types::Value = row.get_unwrap(name.as_str());
            map.insert(name.clone(), sqlite_val_to_json(val));
        }
        Ok(Value::Object(map))
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

pub fn explain(conn: &Connection, query: &Query) -> Result<Vec<Value>, RqlError> {
    let (sql, params) = to_sql(query)?;
    let explain_sql = format!("EXPLAIN QUERY PLAN {sql}");
    let mut stmt = conn.prepare(&explain_sql)?;
    let col_names: Vec<String> = (0..stmt.column_count()).map(|i| stmt.column_name(i).unwrap_or("?").to_string()).collect();
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        let mut map = serde_json::Map::new();
        for name in &col_names {
            let val: rusqlite::types::Value = row.get_unwrap(name.as_str());
            map.insert(name.clone(), sqlite_val_to_json(val));
        }
        Ok(Value::Object(map))
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

fn sqlite_val_to_json(val: rusqlite::types::Value) -> Value {
    match val {
        rusqlite::types::Value::Null => Value::Null,
        rusqlite::types::Value::Integer(i) => Value::Number(i.into()),
        rusqlite::types::Value::Real(f) => {
            serde_json::Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null)
        }
        rusqlite::types::Value::Text(s) => Value::String(s),
        rusqlite::types::Value::Blob(_) => Value::String("[blob]".into()),
    }
}

pub fn to_sql(query: &Query) -> Result<(String, Vec<Box<dyn rusqlite::types::ToSql>>), RqlError> {
    let table = match query.source.to_lowercase().as_str() {
        "events" => "events",
        "sessions" => "sessions",
        s => return Err(RqlError::UnknownSource(s.to_string())),
    };

    let mut sql = String::from("SELECT * FROM ");
    sql.push_str(table);

    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut conditions = Vec::new();

    if let Some(ref filter) = query.filter {
        let (clause, mut p) = condition_to_sql(filter, table)?;
        conditions.push(clause);
        params.append(&mut p);
    }

    if let Some(ref tw) = query.time_window {
        let (clause, mut p) = time_window_to_sql(tw, table);
        conditions.push(clause);
        params.append(&mut p);
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    if !query.group_by.is_empty() {
        let cols: Vec<&str> = query.group_by.iter().map(|s| s.as_str()).collect();
        sql.push_str(" GROUP BY ");
        sql.push_str(&cols.join(", "));
    }

    if !query.order_by.is_empty() {
        let order_clauses: Vec<String> = query.order_by.iter().map(|o| {
            let dir = if o.descending { " DESC" } else { "" };
            format!("{}{}", field_col(&o.field), dir)
        }).collect();
        sql.push_str(" ORDER BY ");
        sql.push_str(&order_clauses.join(", "));
    }

    Ok((sql, params))
}

fn field_col(field: &str) -> String {
    // RQL field names are friendlier than the raw column names.
    let col = match field {
        "id" => "event_id",
        "session_id" => "session_id",
        "sequence" => "sequence",
        "kind" => "kind",
        "severity" => "severity",
        "source" => "source_kind",
        "source_value" => "source_value",
        "timestamp" => "utc_timestamp",
        "duration_ms" => "duration_ns",
        "correlation_id" => "correlation_id",
        "parent_event_id" => "parent_event_id",
        "payload" => "payload_json",
        "classification" => "classification",
        "error" => "is_error_like",
        "project_id" => "project_id",
        "command" => "command",
        "state" => "state",
        "started_at" => "started_at",
        "stopped_at" => "stopped_at",
        "label" => "labels",
        _ => field,
    };
    format!("\"{col}\"")
}

#[allow(clippy::used_underscore_binding)]
fn condition_to_sql(cond: &Condition, _table: &str) -> Result<(String, Vec<Box<dyn rusqlite::types::ToSql>>), RqlError> {
    match cond {
        Condition::Compare { field, op, value } => {
            let col = field_col(field);
            let is_null = matches!(value, Literal::Null);
            let (val_expr, params) = value_to_sql(value);
            let sql = if is_null {
                format!("{col} IS NULL")
            } else {
                format!("{col} {} {val_expr}", op.sql())
            };
            Ok((sql, params))
        }
        Condition::And(left, right) => {
            let (ls, mut lp) = condition_to_sql(left, _table)?;
            let (rs, rp) = condition_to_sql(right, _table)?;
            lp.extend(rp);
            Ok((format!("({ls} AND {rs})"), lp))
        }
        Condition::Or(left, right) => {
            let (ls, mut lp) = condition_to_sql(left, _table)?;
            let (rs, rp) = condition_to_sql(right, _table)?;
            lp.extend(rp);
            Ok((format!("({ls} OR {rs})"), lp))
        }
        Condition::Not(inner) => {
            let (s, p) = condition_to_sql(inner, _table)?;
            Ok((format!("NOT ({s})"), p))
        }
        Condition::Group(inner) => condition_to_sql(inner, _table),
    }
}

fn value_to_sql(val: &Literal) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
    match val {
        Literal::Str(s) => ("?".into(), vec![Box::new(s.clone())]),
        Literal::Num(n) => ("?".into(), vec![Box::new(*n)]),
        Literal::Bool(b) => ("?".into(), vec![Box::new(i32::from(*b))]),
        Literal::Null => ("NULL".into(), vec![]),
        Literal::Field(s) => (field_col(s), vec![]),
    }
}

fn time_window_to_sql(tw: &TimeWindow, table: &str) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
    let dir_op = match tw.direction {
        TimeDirection::Before => "<",
        TimeDirection::After => ">",
    };

    let ts_col = if table == "sessions" { "started_at" } else { "utc_timestamp" };

    let sql = format!(
        "{ts_col} {dir_op} (SELECT {ts_col} FROM {table} WHERE kind = ?1 ORDER BY {ts_col} DESC LIMIT 1)"
    );

    (sql, vec![Box::new(tw.anchor_kind.clone())])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn test_to_sql_simple_where() {
        let q = parser::parse("FROM events WHERE kind = \"test\"").unwrap();
        let (sql, params) = to_sql(&q).unwrap();
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("\"kind\""));
        assert!(params.len() == 1);
    }

    #[test]
    fn test_to_sql_group_order() {
        let q = parser::parse("FROM events WHERE severity = \"error\" GROUP BY kind ORDER BY count DESC").unwrap();
        let (sql, _) = to_sql(&q).unwrap();
        assert!(sql.contains("GROUP BY"));
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("DESC"));
    }

    #[test]
    fn test_to_sql_unknown_source() {
        let q = parser::parse("FROM nonexistent").unwrap();
        assert!(to_sql(&q).is_err());
    }
}
