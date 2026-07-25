use sqlparser::ast::{
    Expr, FunctionArg, FunctionArgExpr, GroupByExpr, JoinConstraint, JoinOperator, Query,
    SelectItem, SetExpr, Statement, TableFactor, TableWithJoins, Value, Values,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

// Turn literal values into ? placeholders so the same query shape
// buckets together regardless of the concrete values being compared.
pub fn normalize(sql: &str) -> Result<String, crate::error::DbError> {
    let dialect = GenericDialect;
    let mut ast = Parser::parse_sql(&dialect, sql)
        .map_err(|e| crate::error::DbError::Parse(e.to_string()))?;
    for stmt in &mut ast {
        replace_literals_in_statement(stmt);
    }
    Ok(ast
        .iter()
        .map(|s| format!("{s}"))
        .collect::<Vec<_>>()
        .join("; "))
}

fn replace_literals_in_expr(expr: &mut Expr) {
    match expr {
        Expr::Value(_) => {
            *expr = Expr::Value(Value::Placeholder("?".to_string()));
        }
        Expr::BinaryOp { left, right, .. } => {
            replace_literals_in_expr(left);
            replace_literals_in_expr(right);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            replace_literals_in_expr(inner);
        }
        Expr::Nested(inner) => {
            replace_literals_in_expr(inner);
        }
        Expr::Between {
            expr: inner,
            low,
            high,
            ..
        } => {
            replace_literals_in_expr(inner);
            replace_literals_in_expr(low);
            replace_literals_in_expr(high);
        }
        Expr::InList {
            expr: inner,
            list,
            ..
        } => {
            replace_literals_in_expr(inner);
            for expr in list.iter_mut() {
                replace_literals_in_expr(expr);
            }
        }
        Expr::Function(func) => {
            for func_arg in func.args.iter_mut() {
                match func_arg {
                    FunctionArg::Named { arg: expr, .. } | FunctionArg::Unnamed(expr) => {
                        if let FunctionArgExpr::Expr(e) = expr {
                            replace_literals_in_expr(e);
                        }
                    }
                }
            }
            if let Some(filter) = &mut func.filter {
                replace_literals_in_expr(filter);
            }
            if let Some(over) = &mut func.over {
                if let sqlparser::ast::WindowType::WindowSpec(spec) = over {
                    for part in spec.partition_by.iter_mut() {
                        replace_literals_in_expr(part);
                    }
                    for order in spec.order_by.iter_mut() {
                        replace_literals_in_expr(&mut order.expr);
                    }
                }
            }
        }
        Expr::Cast { expr: inner, .. } | Expr::Extract { expr: inner, .. } => {
            replace_literals_in_expr(inner);
        }
        Expr::Subquery(_) => {}
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
            ..
        } => {
            if let Some(op) = operand {
                replace_literals_in_expr(op);
            }
            for cond in conditions.iter_mut() {
                replace_literals_in_expr(cond);
            }
            for case_result in results.iter_mut() {
                replace_literals_in_expr(case_result);
            }
            if let Some(el) = else_result {
                replace_literals_in_expr(el);
            }
        }
        Expr::Tuple(items) => {
            for expr in items.iter_mut() {
                replace_literals_in_expr(expr);
            }
        }
        Expr::Array(arr) => {
            for elem in arr.elem.iter_mut() {
                replace_literals_in_expr(elem);
            }
        }
        Expr::SimilarTo {
            expr: inner,
            pattern,
            ..
        }
        | Expr::Like {
            expr: inner,
            pattern,
            ..
        }
        | Expr::ILike {
            expr: inner,
            pattern,
            ..
        }
        | Expr::RLike {
            expr: inner,
            pattern,
            ..
        } => {
            replace_literals_in_expr(inner);
            replace_literals_in_expr(pattern);
        }
        Expr::IsNull(_)
        | Expr::IsNotNull(_)
        | Expr::IsTrue(_)
        | Expr::IsNotTrue(_)
        | Expr::IsFalse(_)
        | Expr::IsNotFalse(_)
        | Expr::IsUnknown(_)
        | Expr::IsNotUnknown(_) => {}
        Expr::IsDistinctFrom(left, right) | Expr::IsNotDistinctFrom(left, right) => {
            replace_literals_in_expr(left);
            replace_literals_in_expr(right);
        }
        Expr::InSubquery { expr: inner, .. } | Expr::InUnnest { expr: inner, .. } => {
            replace_literals_in_expr(inner);
        }
        Expr::AnyOp { left, right, .. } | Expr::AllOp { left, right, .. } => {
            replace_literals_in_expr(left);
            replace_literals_in_expr(right);
        }
        Expr::Exists { .. } => {}
        Expr::Substring {
            expr: inner,
            substring_from,
            substring_for,
            ..
        } => {
            replace_literals_in_expr(inner);
            if let Some(from) = substring_from {
                replace_literals_in_expr(from);
            }
            if let Some(for_expr) = substring_for {
                replace_literals_in_expr(for_expr);
            }
        }
        Expr::Trim {
            expr: inner,
            trim_what,
            ..
        } => {
            replace_literals_in_expr(inner);
            if let Some(what) = trim_what {
                replace_literals_in_expr(what);
            }
        }
        Expr::Overlay {
            expr: inner,
            overlay_what,
            overlay_from,
            overlay_for,
        } => {
            replace_literals_in_expr(inner);
            replace_literals_in_expr(overlay_what);
            replace_literals_in_expr(overlay_from);
            if let Some(for_expr) = overlay_for {
                replace_literals_in_expr(for_expr);
            }
        }
        Expr::Collate { expr: inner, .. } => {
            replace_literals_in_expr(inner);
        }
        Expr::Position { expr: inner, r#in } => {
            replace_literals_in_expr(inner);
            replace_literals_in_expr(r#in);
        }
        Expr::AtTimeZone { timestamp, time_zone: _ } => {
            replace_literals_in_expr(timestamp);
        }
        Expr::Ceil { expr: inner, .. } | Expr::Floor { expr: inner, .. } => {
            replace_literals_in_expr(inner);
        }
        Expr::Convert { .. } | Expr::IntroducedString { .. } | Expr::TypedString { .. } => {}
        _ => {}
    }
}

fn replace_literals_in_statement(stmt: &mut Statement) {
    match stmt {
        Statement::Query(query) => {
            replace_literals_in_query(query);
        }
        Statement::Insert {
            source: insert_source,
            ..
        } => {
            if let Some(src) = insert_source {
                replace_literals_in_query(src);
            }
        }
        Statement::Update { selection, .. } => {
            if let Some(expr) = selection {
                replace_literals_in_expr(expr);
            }
        }
        Statement::Delete {
            selection: delete_selection,
            ..
        } => {
            if let Some(expr) = delete_selection {
                replace_literals_in_expr(expr);
            }
        }
        _ => {}
    }
}

fn replace_literals_in_query(query: &mut Query) {
    replace_literals_in_set_expr(&mut query.body);
    for order in &mut query.order_by {
        replace_literals_in_expr(&mut order.expr);
    }
    if let Some(expr) = &mut query.limit {
        replace_literals_in_expr(expr);
    }
    if let Some(offset) = &mut query.offset {
        replace_literals_in_expr(&mut offset.value);
    }
}

fn replace_literals_in_set_expr(set_expr: &mut SetExpr) {
    match set_expr {
        SetExpr::Select(select) => {
            for select_item in &mut select.projection {
                match select_item {
                    SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } => {
                        replace_literals_in_expr(expr);
                    }
                    _ => {}
                }
            }
            for twj in &mut select.from {
                replace_literals_in_table_with_joins(twj);
            }
            if let Some(expr) = &mut select.selection {
                replace_literals_in_expr(expr);
            }
            if let GroupByExpr::Expressions(exprs) = &mut select.group_by {
                for expr in exprs.iter_mut() {
                    replace_literals_in_expr(expr);
                }
            }
            if let Some(expr) = &mut select.having {
                replace_literals_in_expr(expr);
            }
        }
        SetExpr::Values(Values { rows, .. }) => {
            for row in rows.iter_mut() {
                for expr in row.iter_mut() {
                    replace_literals_in_expr(expr);
                }
            }
        }
        SetExpr::Query(subquery) => {
            replace_literals_in_query(subquery);
        }
        SetExpr::SetOperation { left, right, .. } => {
            replace_literals_in_set_expr(left);
            replace_literals_in_set_expr(right);
        }
        _ => {}
    }
}

fn replace_literals_in_table_with_joins(twj: &mut TableWithJoins) {
    replace_literals_in_table_factor(&mut twj.relation);
    for join in &mut twj.joins {
        let constraint = match &mut join.join_operator {
            JoinOperator::Inner(c)
            | JoinOperator::LeftOuter(c)
            | JoinOperator::RightOuter(c)
            | JoinOperator::FullOuter(c)
            | JoinOperator::LeftSemi(c)
            | JoinOperator::RightSemi(c)
            | JoinOperator::LeftAnti(c)
            | JoinOperator::RightAnti(c) => Some(c),
            _ => None,
        };
        if let Some(JoinConstraint::On(expr)) = constraint {
            replace_literals_in_expr(expr);
        }
    }
}

fn replace_literals_in_table_factor(tf: &mut TableFactor) {
    match tf {
        TableFactor::Derived { subquery, .. } => {
            replace_literals_in_query(subquery.as_mut());
        }
        TableFactor::TableFunction { expr, .. } => {
            replace_literals_in_expr(expr);
        }
        TableFactor::Function { args, .. } => {
            for func_arg in args.iter_mut() {
                match func_arg {
                    FunctionArg::Named { arg: expr, .. } | FunctionArg::Unnamed(expr) => {
                        if let FunctionArgExpr::Expr(e) = expr {
                            replace_literals_in_expr(e);
                        }
                    }
                }
            }
        }
        TableFactor::UNNEST { array_exprs, .. } => {
            for expr in array_exprs.iter_mut() {
                replace_literals_in_expr(expr);
            }
        }
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            replace_literals_in_table_with_joins(table_with_joins);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_where_literals() {
        let sql = "SELECT * FROM users WHERE id = 1 AND name = 'alice'";
        let normalized = normalize(sql).unwrap();
        assert_eq!(
            normalized,
            "SELECT * FROM users WHERE id = ? AND name = ?"
        );
    }

    #[test]
    fn test_insert_values() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'alice')";
        let normalized = normalize(sql).unwrap();
        assert!(normalized.contains("?"));
        assert!(normalized.contains("VALUES"));
    }

    #[test]
    fn test_in_list() {
        let sql = "SELECT * FROM users WHERE id IN (1, 2, 3)";
        let normalized = normalize(sql).unwrap();
        assert!(normalized.contains("IN ("));
        assert!(normalized.contains("?"));
    }

    #[test]
    fn test_select_without_literals() {
        let sql = "SELECT COUNT(*) FROM users";
        let normalized = normalize(sql).unwrap();
        assert_eq!(normalized, sql);
    }
}
