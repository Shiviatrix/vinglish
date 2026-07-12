use eng_lexer::Span;

// ─────────────────────────────────────────────────────────────────────────────
// Identifiers
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}

impl std::fmt::Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Visibility
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Private,
    Public,
    Internal,
}

// ─────────────────────────────────────────────────────────────────────────────
// Type expressions (as written in source)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// `number`, `text`, `boolean`, `decimal`, or any named type
    Named(Ident),
    /// `List of T`
    List(Box<TypeExpr>),
    /// `Dictionary from K to V`
    Dict {
        key: Box<TypeExpr>,
        val: Box<TypeExpr>,
    },
    /// `Optional T` / `T?`
    Optional(Box<TypeExpr>),
    /// `Result of T` (error type is inferred)
    Result(Box<TypeExpr>),
    /// Generic instantiation: `Array of length N` etc.
    Generic { base: Ident, args: Vec<TypeExpr> },
    /// `borrow T` or `borrow mutable T`
    Reference { mutable: bool, inner: Box<TypeExpr> },
}

// ─────────────────────────────────────────────────────────────────────────────
// Patterns (for `match`)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// `case CreditCard`
    Constructor(Ident),
    /// `case x` where x is a name binding
    Bind(Ident),
    /// `case 42`
    Literal(Literal),
    /// Wildcard `_`
    Wildcard(Span),
}

// ─────────────────────────────────────────────────────────────────────────────
// Literals
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Unit,
}

// ─────────────────────────────────────────────────────────────────────────────
// Expressions
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal value
    Lit { value: Literal, span: Span },
    /// A simple identifier reference
    Ident(Ident),
    /// Generic instantiation: `vector_new<number>`
    GenericInst {
        base: Ident,
        args: Vec<TypeExpr>,
        span: Span,
    },
    /// Function call: `f(a, b)` or `calculate tax for order`
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Binary operation: `a + b`, `balance is below 0`
    BinOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },
    /// Unary operation: `not x`, `-x`
    UnOp {
        op: UnOp,
        operand: Box<Expr>,
        span: Span,
    },
    /// Field access: `account.balance`
    Field {
        object: Box<Expr>,
        field: Ident,
        span: Span,
    },
    /// Index: `list[i]`
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// Struct Literal: `Point { x: 10, y: 20 }` or `Pair<number> { first: 10, second: 20 }`
    StructLit {
        ty: Box<Expr>,
        fields: Vec<(Ident, Expr)>,
        span: Span,
    },
    /// A block used as an expression (rare but valid)
    Block(Box<Block>),
    /// List literal: `[1, 2, 3]`
    List { elements: Vec<Expr>, span: Span },
    /// Macro call: `fmt!(...)`
    MacroCall {
        name: Ident,
        args: Vec<Expr>,
        span: Span,
    },
    /// Postfix Try `?`
    PostfixTry {
        inner: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Lit { span, .. }
            | Expr::Call { span, .. }
            | Expr::BinOp { span, .. }
            | Expr::UnOp { span, .. }
            | Expr::Field { span, .. }
            | Expr::Index { span, .. }
            | Expr::StructLit { span, .. }
            | Expr::List { span, .. }
            | Expr::MacroCall { span, .. }
            | Expr::PostfixTry { span, .. }
            | Expr::GenericInst { span, .. } => *span,
            Expr::Ident(id) => id.span,
            Expr::Block(b) => b.span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    IsBelow,
    IsAbove,
    Exceeds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    Neg,
    Not,
    Deref,
    Borrow(bool), // true if mutable
}

// ─────────────────────────────────────────────────────────────────────────────
// Statements
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let x be 5`  or  `let x be number`
    Let(LetStmt),
    /// `return expr`
    Return(ReturnStmt),
    /// `if cond then ... otherwise ...`
    If(IfStmt),
    /// `when cond ... otherwise ...`
    When(WhenStmt),
    /// `repeat for every x ...` or `repeat while cond ...`
    Repeat(RepeatStmt),
    /// `parallel repeat for every x ...`
    ParallelRepeat(RepeatStmt),
    /// `match x case A ... otherwise ...`
    Match(MatchStmt),
    /// `account.balance += amount`
    Assign(AssignStmt),
    /// `spawn Worker`
    Spawn(SpawnStmt),
    /// `send task to worker`
    Send(SendStmt),
    /// `receive result`
    Receive(ReceiveStmt),
    /// `transaction ... commit`
    Transaction(TransactionStmt),
    /// A bare expression statement (e.g., `print("hello")`)
    Expr(Expr),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let(s) => s.span,
            Stmt::Return(s) => s.span,
            Stmt::If(s) => s.span,
            Stmt::When(s) => s.span,
            Stmt::Repeat(s) | Stmt::ParallelRepeat(s) => s.span(),
            Stmt::Match(s) => s.span,
            Stmt::Assign(s) => s.span,
            Stmt::Spawn(s) => s.span,
            Stmt::Send(s) => s.span,
            Stmt::Receive(s) => s.span,
            Stmt::Transaction(s) => s.span,
            Stmt::Expr(e) => e.span(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetStmt {
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub value: Option<Expr>,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_block: Block,
    pub otherwise: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhenStmt {
    pub condition: Expr,
    pub then_block: Block,
    pub otherwise: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepeatStmt {
    ForEvery {
        var: Ident,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    Count {
        times: Expr,
        body: Block,
        span: Span,
    },
}

impl RepeatStmt {
    pub fn span(&self) -> Span {
        match self {
            RepeatStmt::ForEvery { span, .. }
            | RepeatStmt::While { span, .. }
            | RepeatStmt::Count { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchStmt {
    pub subject: Expr,
    pub cases: Vec<MatchCase>,
    pub otherwise: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignStmt {
    pub target: Expr,
    pub op: AssignOp,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnStmt {
    pub actor: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SendStmt {
    pub message: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiveStmt {
    pub binding: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransactionStmt {
    pub body: Block,
    pub span: Span,
}

// ─────────────────────────────────────────────────────────────────────────────
// Blocks
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

impl Block {
    pub fn empty(span: Span) -> Self {
        Self {
            stmts: vec![],
            span,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Top-level items
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub ty: TypeExpr,
    pub name: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub visibility: Visibility,
    pub is_foreign: bool,
    pub name: Ident,
    pub type_params: Vec<Ident>,
    pub target_type: Option<Ident>, // e.g. `on Point`
    pub params: Vec<Param>,
    pub ret_type: Option<TypeExpr>,
    pub effects: Vec<Ident>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub visibility: Visibility,
    pub name: Ident,
    pub type_params: Vec<Ident>,
    pub fields: Vec<Param>,
    pub capabilities: Vec<Ident>, // `requires draw(), serialize()`
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageDecl {
    pub name: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    pub name: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseDecl {
    pub path: Vec<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteDecl {
    pub path: String,
    pub handler: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: Ident,
    pub payload: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub visibility: Visibility,
    pub name: Ident,
    pub type_params: Vec<Ident>,
    pub variants: Vec<Variant>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(FunctionDef),
    Type(TypeDef),
    Enum(EnumDef),
    Package(PackageDecl),
    Module(ModuleDecl),
    Use(UseDecl),
    Route(RouteDecl),
    /// Top-level statement (script mode)
    Statement(Stmt),
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::Function(f) => f.span,
            Item::Type(t) => t.span,
            Item::Enum(e) => e.span,
            Item::Package(p) => p.span,
            Item::Module(m) => m.span,
            Item::Use(u) => u.span,
            Item::Route(r) => r.span,
            Item::Statement(s) => s.span(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Module (root of the AST)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    pub items: Vec<Item>,
    pub span: Span,
}

impl std::cmp::Eq for Literal {}
impl std::hash::Hash for Literal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Literal::Int(n) => n.hash(state),
            Literal::Float(f) => f.to_bits().hash(state),
            Literal::Text(s) => s.hash(state),
            Literal::Bool(b) => b.hash(state),
            Literal::Unit => (),
        }
    }
}
