use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub source: String,
    pub filter: Option<Condition>,
    pub time_window: Option<TimeWindow>,
    pub group_by: Vec<String>,
    pub order_by: Vec<OrderExpr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    Compare {
        field: String,
        op: ComparisonOp,
        value: Literal,
    },
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
    Group(Box<Condition>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Like,
}

impl ComparisonOp {
    pub const fn sql(&self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::Like => "LIKE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    Field(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeWindow {
    pub duration_ms: i64,
    pub direction: TimeDirection,
    pub anchor_kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeDirection {
    Before,
    After,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderExpr {
    pub field: String,
    pub descending: bool,
}
