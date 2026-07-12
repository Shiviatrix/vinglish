pub mod types;
pub mod symbol;

use eng_lexer::span::Span;
use eng_parser::ast::{BinOp, UnOp, Literal, Visibility};
use crate::symbol::{TypeId, FunctionId, VariableId, FieldId};

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(FunctionDef),
    Type(TypeDef),
    Statement(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub visibility: Visibility,
    pub is_foreign: bool,
    pub id: FunctionId,
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: TypeId,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub id: VariableId,
    pub name: String,
    pub ty: TypeId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub visibility: Visibility,
    pub id: TypeId,
    pub name: String,
    pub fields: Vec<Param>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        id: VariableId,
        name: String,
        is_mut: bool,
        ty: TypeId,
        init: Expr,
        span: Span,
    },
    Assign {
        target: Expr,
        op: eng_parser::ast::AssignOp,
        value: Expr,
        span: Span,
    },
    If {
        condition: Expr,
        then_block: Block,
        otherwise: Option<Block>,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    RepeatWhile {
        condition: Expr,
        body: Block,
        span: Span,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Lit {
        value: Literal,
        ty: TypeId,
        span: Span,
    },
    VarRef {
        id: VariableId,
        ty: TypeId,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        ty: TypeId,
        span: Span,
    },
    BinOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        ty: TypeId,
        span: Span,
    },
    UnOp {
        op: UnOp,
        operand: Box<Expr>,
        ty: TypeId,
        span: Span,
    },
    FieldIndex {
        object: Box<Expr>,
        field_id: FieldId,
        ty: TypeId,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        ty: TypeId,
        span: Span,
    },
    List {
        elements: Vec<Expr>,
        ty: TypeId,
        span: Span,
    },
    StructInit {
        id: TypeId,
        fields: Vec<Expr>,
        ty: TypeId,
        span: Span,
    },
    Block(Block),
}

impl Expr {
    pub fn ty(&self) -> TypeId {
        match self {
            Expr::Lit { ty, .. } => *ty,
            Expr::VarRef { ty, .. } => *ty,
            Expr::Call { ty, .. } => *ty,
            Expr::BinOp { ty, .. } => *ty,
            Expr::UnOp { ty, .. } => *ty,
            Expr::FieldIndex { ty, .. } => *ty,
            Expr::Index { ty, .. } => *ty,
            Expr::List { ty, .. } => *ty,
            Expr::StructInit { ty, .. } => *ty,
            Expr::Block(b) => b.ty,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Expr::Lit { span, .. } => *span,
            Expr::VarRef { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::BinOp { span, .. } => *span,
            Expr::UnOp { span, .. } => *span,
            Expr::FieldIndex { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::List { span, .. } => *span,
            Expr::StructInit { span, .. } => *span,
            Expr::Block(b) => b.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub expr: Option<Box<Expr>>,
    pub ty: TypeId,
    pub span: Span,
}
