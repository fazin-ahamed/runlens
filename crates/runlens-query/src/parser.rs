use crate::ast::*;
use crate::error::RqlError;
use crate::lexer::{Lexer, Token};

pub fn parse(input: &str) -> Result<Query, RqlError> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_query()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        self.pos += 1;
        t
    }

    fn expect(&mut self, expected: &Token) -> Result<(), RqlError> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(expected) {
            self.advance();
            Ok(())
        } else {
            Err(RqlError::parse(self.pos, format!("expected {expected:?}, got {:?}", self.peek())))
        }
    }

    fn expect_ident(&mut self) -> Result<String, RqlError> {
        let pos = self.pos;
        match self.advance() {
            Token::Ident(s) => Ok(s.clone()),
            t => Err(RqlError::parse(pos, format!("expected identifier, got {t:?}"))),
        }
    }

    fn expect_string(&mut self) -> Result<String, RqlError> {
        let pos = self.pos;
        match self.advance() {
            Token::String(s) => Ok(s.clone()),
            t => Err(RqlError::parse(pos, format!("expected string, got {t:?}"))),
        }
    }

    fn parse_query(&mut self) -> Result<Query, RqlError> {
        self.expect(&Token::From)?;
        let source = self.expect_ident()?;

        let filter = if self.peek() == &Token::Where {
            self.advance();
            Some(self.parse_condition()?)
        } else {
            None
        };

        let time_window = if self.peek() == &Token::Within {
            self.advance();
            Some(self.parse_time_window()?)
        } else {
            None
        };

        let group_by = if self.peek() == &Token::Group {
            self.advance();
            self.expect(&Token::By)?;
            let mut fields = Vec::new();
            loop {
                fields.push(self.expect_ident()?);
                if self.peek() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
            fields
        } else {
            Vec::new()
        };

        let order_by = if self.peek() == &Token::Order {
            self.advance();
            self.expect(&Token::By)?;
            let mut exprs = Vec::new();
            loop {
                exprs.push(self.parse_order_expr()?);
                if self.peek() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
            exprs
        } else {
            Vec::new()
        };

        Ok(Query { source, filter, time_window, group_by, order_by })
    }

    fn parse_condition(&mut self) -> Result<Condition, RqlError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Condition, RqlError> {
        let mut left = self.parse_and()?;
        while self.peek() == &Token::Or {
            self.advance();
            let right = self.parse_and()?;
            left = Condition::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Condition, RqlError> {
        let mut left = self.parse_not()?;
        while self.peek() == &Token::And {
            self.advance();
            let right = self.parse_not()?;
            left = Condition::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Condition, RqlError> {
        if self.peek() == &Token::Not {
            self.advance();
            let inner = self.parse_primary()?;
            Ok(Condition::Not(Box::new(inner)))
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Condition, RqlError> {
        if self.peek() == &Token::LParen {
            self.advance();
            let cond = self.parse_condition()?;
            self.expect(&Token::RParen)?;
            Ok(Condition::Group(Box::new(cond)))
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> Result<Condition, RqlError> {
        let field = self.expect_ident()?;
        if self.peek() == &Token::Dot {
            self.advance();
            let sub = self.expect_ident()?;
            let op = self.parse_op()?;
            let value = self.parse_value()?;
            Ok(Condition::Compare { field: format!("{field}.{sub}"), op, value })
        } else {
            let op = self.parse_op()?;
            let value = self.parse_value()?;
            Ok(Condition::Compare { field, op, value })
        }
    }

    fn parse_op(&mut self) -> Result<ComparisonOp, RqlError> {
        let pos = self.pos;
        match self.advance() {
            Token::Eq => Ok(ComparisonOp::Eq),
            Token::Ne => Ok(ComparisonOp::Ne),
            Token::Lt => Ok(ComparisonOp::Lt),
            Token::Gt => Ok(ComparisonOp::Gt),
            Token::Le => Ok(ComparisonOp::Le),
            Token::Ge => Ok(ComparisonOp::Ge),
            Token::Like => Ok(ComparisonOp::Like),
            t => Err(RqlError::parse(pos, format!("expected comparison operator, got {t:?}"))),
        }
    }

    fn parse_value(&mut self) -> Result<Literal, RqlError> {
        let pos = self.pos;
        match self.advance() {
            Token::String(s) => Ok(Literal::Str(s.clone())),
            Token::Number(n, _) => Ok(Literal::Num(*n)),
            Token::True => Ok(Literal::Bool(true)),
            Token::False => Ok(Literal::Bool(false)),
            Token::Null => Ok(Literal::Null),
            Token::Ident(s) => Ok(Literal::Field(s.clone())),
            t => Err(RqlError::parse(pos, format!("expected literal, got {t:?}"))),
        }
    }

    fn parse_time_window(&mut self) -> Result<TimeWindow, RqlError> {
        let pos1 = self.pos;
        let (num, _unit) = match self.advance() {
            Token::Number(n, Some(u)) => {
                let ms = parse_duration_ms(*n, u)?;
                (ms, u.clone())
            }
            Token::Number(n, None) => {
                let ms = parse_duration_ms(*n, "s")?;
                (ms, "s".into())
            }
            t => return Err(RqlError::parse(pos1, format!("expected duration (number + unit), got {t:?}"))),
        };

        let pos2 = self.pos;
        let direction = match self.advance() {
            Token::Before => TimeDirection::Before,
            Token::After => TimeDirection::After,
            t => return Err(RqlError::parse(pos2, format!("expected BEFORE or AFTER, got {t:?}"))),
        };

        let anchor = self.parse_anchor()?;

        Ok(TimeWindow { duration_ms: num, direction, anchor_kind: anchor })
    }

    fn parse_anchor(&mut self) -> Result<String, RqlError> {
        match self.peek() {
            Token::Ident(_) => {
                let _name = self.expect_ident()?;
                self.expect(&Token::LParen)?;
                let val = self.expect_string()?;
                self.expect(&Token::RParen)?;
                Ok(val)
            }
            Token::String(_) => self.expect_string(),
            t => Err(RqlError::parse(self.pos, format!("expected anchor (string or function), got {t:?}"))),
        }
    }

    fn parse_order_expr(&mut self) -> Result<OrderExpr, RqlError> {
        let field = self.expect_ident()?;
        let descending = if self.peek() == &Token::Desc {
            self.advance();
            true
        } else if self.peek() == &Token::Asc {
            self.advance();
            false
        } else {
            false
        };
        Ok(OrderExpr { field, descending })
    }
}

fn parse_duration_ms(num: f64, unit: &str) -> Result<i64, RqlError> {
    match unit.to_lowercase().as_str() {
        "ms" => Ok(num as i64),
        "s" => Ok((num * 1000.0) as i64),
        "m" => Ok((num * 60_000.0) as i64),
        "h" => Ok((num * 3_600_000.0) as i64),
        u => Err(RqlError::parse(0, format!("unknown time unit '{u}' (use ms, s, m, h)"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let q = parse("FROM events WHERE kind = \"network.response\"").unwrap();
        assert_eq!(q.source, "events");
        assert!(q.filter.is_some());
    }

    #[test]
    fn test_parse_group_order() {
        let q = parse("FROM events WHERE severity = \"error\" GROUP BY kind ORDER BY count DESC").unwrap();
        assert_eq!(q.source, "events");
        assert_eq!(q.group_by, vec!["kind"]);
        assert_eq!(q.order_by.len(), 1);
        assert!(q.order_by[0].descending);
    }

    #[test]
    fn test_parse_time_window() {
        let q = parse("FROM events WITHIN 5s BEFORE \"crash\"").unwrap();
        let tw = q.time_window.unwrap();
        assert_eq!(tw.duration_ms, 5000);
        assert_eq!(tw.anchor_kind, "crash");
    }

    #[test]
    fn test_parse_and_condition() {
        let q = parse("FROM events WHERE kind = \"error\" AND severity >= 3").unwrap();
        let cond = q.filter.unwrap();
        match cond {
            Condition::And(_, _) => {}
            _ => panic!("expected AND condition"),
        }
    }

    #[test]
    fn test_parse_parse_error() {
        let r = parse("FROM");
        assert!(r.is_err());
    }
}
