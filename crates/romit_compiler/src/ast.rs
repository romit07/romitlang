use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Program {
    Spec(Spec),
    Script(Script),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub stmts: Vec<ScriptStmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptStmt {
    Say {
        parts: Vec<Expr>,
        span: Span,
    },
    Set {
        name: String,
        value: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spec {
    pub name: String,
    pub version: i64,
    pub about: Option<String>,
    pub decls: Vec<Decl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Enum(EnumDecl),
    Config(ConfigDecl),
    State(StateDecl),
    Entity(EntityDecl),
    Event(EventDecl),
    Output(OutputDecl),
    Define(DefineDecl),
    Machine(MachineDecl),
    Handler(HandlerDecl),
    Invariant(InvariantDecl),
    Limit(LimitDecl),
    Test(TestDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigDecl {
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateDecl {
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityDecl {
    pub name: String,
    pub key: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub compare: Option<(CompareOp, Literal)>,
    pub default: Option<Literal>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub stores: StoreMode,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StoreMode {
    None,
    All,
    Named(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DefineDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Type,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MachineDecl {
    pub path: PathExpr,
    pub transitions: Vec<MachineTransition>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MachineTransition {
    pub from: String,
    pub to: Vec<String>,
    pub final_state: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HandlerDecl {
    pub event: String,
    pub bindings: Vec<String>,
    pub condition: Option<Expr>,
    pub run_all: bool,
    pub rules: Vec<RuleDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleDecl {
    pub name: String,
    pub when: RuleWhen,
    pub actions: Vec<Action>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuleWhen {
    Expr(Expr),
    Otherwise,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Set {
        path: PathExpr,
        value: Expr,
        span: Span,
    },
    Emit {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    Reject {
        message: String,
        span: Span,
    },
    Say {
        parts: Vec<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvariantDecl {
    pub name: String,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LimitDecl {
    pub output: String,
    pub max: i64,
    pub per: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestStep {
    Send(Send),
    Expect(Expect),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestDecl {
    pub name: String,
    pub givens: Vec<Given>,
    pub steps: Vec<TestStep>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Given {
    EntityInstance {
        entity: String,
        fields: BTreeMap<String, Expr>,
        span: Span,
    },
    StateAssign {
        name: String,
        value: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Send {
    pub event: String,
    pub args: BTreeMap<String, Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expect {
    Expr {
        expr: Expr,
        span: Span,
    },
    Emitted {
        output: String,
        args: Vec<Expr>,
        span: Span,
    },
    Rejected {
        message: String,
        span: Span,
    },
    NoOutputs {
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Named(String),
    Span(Span),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal, Span),
    Path(PathExpr, Span),
    Call {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_br: Box<Expr>,
        else_br: Box<Expr>,
        span: Span,
    },
    Aggregate {
        kind: AggregateKind,
        entity: String,
        field: Option<String>,
        filter: Option<Box<Expr>>,
        span: Span,
    },
    Index {
        entity: String,
        key: Box<Expr>,
        field: String,
        span: Span,
    },
    ForEach {
        var: String,
        entity: String,
        body: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggregateKind {
    Count,
    Sum,
    Min,
    Max,
    Any,
    All,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    In,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathExpr {
    pub parts: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Decimal(String),
    Text(String),
    Bool(bool),
    Enum(String),
}
