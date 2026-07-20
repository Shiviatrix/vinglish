//! Stable, versioned interchange exported by the Vinglish compiler.
//!
//! The compiler's HIR is deliberately *not* serialized. HIR is an internal
//! implementation detail and can evolve with the compiler. This crate lowers
//! HIR into a small transport model that external tools can rely on instead.

use serde::{Deserialize, Serialize};
use vinglish_hir::{
    symbol::{SymbolKind, SymbolTable, TypeId, VariableId},
    types::Type,
    Block, Expr, FunctionDef, Item, Module as HirModule, Stmt,
};
use vinglish_lexer::Span;
use vinglish_parser::ast::{AssignOp, BinOp, Literal, UnOp};

pub const FORMAT_NAME: &str = "vinglish.semantic-export";
pub const CURRENT_VERSION: u32 = 1;

/// The complete external document emitted by `ving --emit-ir`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportDocument {
    pub format: String,
    pub version: u32,
    pub program: Program,
}

impl ExportDocument {
    pub fn new(modules: Vec<Module>) -> Self {
        Self {
            format: FORMAT_NAME.to_owned(),
            version: CURRENT_VERSION,
            program: Program { modules },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub functions: Vec<Function>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    #[serde(rename = "return_type")]
    pub return_type: ExportType,
    pub body: Vec<Statement>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub foreign: bool,
    pub span: SourceRange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ExportType,
    pub span: SourceRange,
}

/// A byte range in the source file. File identity remains outside the document
/// so callers can choose their own source registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start: u32,
    pub end: u32,
}

impl From<Span> for SourceRange {
    fn from(value: Span) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

/// Semantic types intentionally avoid Vinglish compiler type identifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportType {
    Integer,
    Decimal,
    Boolean,
    Text,
    Unit,
    Collection {
        element: Box<ExportType>,
    },
    Map {
        key: Box<ExportType>,
        value: Box<ExportType>,
    },
    Optional {
        inner: Box<ExportType>,
    },
    Result {
        ok: Box<ExportType>,
        err: Box<ExportType>,
    },
    Reference {
        mutable: bool,
        inner: Box<ExportType>,
    },
    Pointer {
        inner: Box<ExportType>,
    },
    Function {
        parameters: Vec<ExportType>,
        returns: Box<ExportType>,
    },
    Named {
        name: String,
        arguments: Vec<ExportType>,
    },
    Unknown,
}

/// The intentionally small v1 statement vocabulary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Statement {
    Variable {
        name: String,
        mutable: bool,
        #[serde(rename = "type")]
        ty: ExportType,
        initializer: Expression,
        span: SourceRange,
    },
    Assignment {
        target: Expression,
        value: Expression,
        span: SourceRange,
    },
    Mutation {
        target: Expression,
        operation: MutationOperation,
        value: Expression,
        span: SourceRange,
    },
    Call {
        call: Call,
        span: SourceRange,
    },
    Return {
        value: Option<Expression>,
        span: SourceRange,
    },
    Loop {
        loop_kind: LoopKind,
        body: Vec<Statement>,
        span: SourceRange,
    },
    Conditional {
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
        span: SourceRange,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LoopKind {
    While { condition: Expression },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
    pub span: SourceRange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expression {
    Identifier {
        name: String,
        span: SourceRange,
    },
    Literal {
        value: LiteralValue,
        span: SourceRange,
    },
    Call(Call),
    Binary {
        operation: BinaryOperation,
        left: Box<Expression>,
        right: Box<Expression>,
        span: SourceRange,
    },
    Unary {
        operation: UnaryOperation,
        operand: Box<Expression>,
        span: SourceRange,
    },
    Collection {
        elements: Vec<Expression>,
        span: SourceRange,
    },
    /// Retains source location when v1 intentionally has no concept for an
    /// internal HIR expression. No internal representation leaks here.
    Unsupported {
        span: SourceRange,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum LiteralValue {
    Integer(i64),
    Decimal(f64),
    Text(String),
    Boolean(bool),
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOperation {
    Negate,
    Not,
}

/// Converts internal HIR into the stable transport model.
///
/// This is the only code that understands both representations. Consumers of
/// the JSON contract never receive a HIR value or compiler symbol identifier.
pub struct ExportBuilder<'a> {
    symbols: &'a SymbolTable,
}

impl<'a> ExportBuilder<'a> {
    pub fn new(symbols: &'a SymbolTable) -> Self {
        Self { symbols }
    }

    pub fn document<I, S>(&self, modules: I) -> ExportDocument
    where
        I: IntoIterator<Item = (S, &'a HirModule)>,
        S: Into<String>,
    {
        ExportDocument::new(
            modules
                .into_iter()
                .map(|(name, module)| self.module(name.into(), module))
                .collect(),
        )
    }

    pub fn module(&self, name: String, module: &HirModule) -> Module {
        let mut functions = Vec::new();
        let mut statements = Vec::new();

        for item in &module.items {
            match item {
                Item::Function(function) => functions.push(self.function(function)),
                Item::Statement(statement) => {
                    if let Some(statement) = self.statement(statement) {
                        statements.push(statement);
                    }
                }
                Item::Type(_) | Item::Enum(_) => {}
            }
        }

        Module {
            name,
            functions,
            statements,
        }
    }

    fn function(&self, function: &FunctionDef) -> Function {
        Function {
            name: function.name.clone(),
            parameters: function
                .params
                .iter()
                .map(|parameter| Parameter {
                    name: parameter.name.clone(),
                    ty: self.type_for(parameter.ty),
                    span: parameter.span.into(),
                })
                .collect(),
            return_type: self.type_for(function.ret_ty),
            body: self.block(&function.body),
            foreign: function.is_foreign,
            span: function.span.into(),
        }
    }

    fn block(&self, expression: &Expr) -> Vec<Statement> {
        match expression {
            Expr::Block(block) => self.block_contents(block),
            expression => self.expression_statement(expression).into_iter().collect(),
        }
    }

    fn block_contents(&self, block: &Block) -> Vec<Statement> {
        let mut statements: Vec<_> = block
            .stmts
            .iter()
            .filter_map(|statement| self.statement(statement))
            .collect();
        if let Some(expression) = &block.expr {
            if let Some(statement) = self.expression_statement(expression) {
                statements.push(statement);
            }
        }
        statements
    }

    fn statement(&self, statement: &Stmt) -> Option<Statement> {
        match statement {
            Stmt::Let {
                name,
                is_mut,
                ty,
                init,
                span,
                ..
            } => Some(Statement::Variable {
                name: name.clone(),
                mutable: *is_mut,
                ty: self.type_for(*ty),
                initializer: self.expression(init),
                span: (*span).into(),
            }),
            Stmt::Assign {
                target,
                op,
                value,
                span,
            } => match op {
                AssignOp::Assign => Some(Statement::Assignment {
                    target: self.expression(target),
                    value: self.expression(value),
                    span: (*span).into(),
                }),
                AssignOp::AddAssign
                | AssignOp::SubAssign
                | AssignOp::MulAssign
                | AssignOp::DivAssign => Some(Statement::Mutation {
                    target: self.expression(target),
                    operation: match op {
                        AssignOp::AddAssign => MutationOperation::Add,
                        AssignOp::SubAssign => MutationOperation::Subtract,
                        AssignOp::MulAssign => MutationOperation::Multiply,
                        AssignOp::DivAssign => MutationOperation::Divide,
                        AssignOp::Assign => unreachable!("handled above"),
                    },
                    value: self.expression(value),
                    span: (*span).into(),
                }),
            },
            Stmt::If {
                condition,
                then_block,
                otherwise,
                span,
            } => Some(Statement::Conditional {
                condition: self.expression(condition),
                then_body: self.block_contents(then_block),
                else_body: otherwise.as_ref().map(|block| self.block_contents(block)),
                span: (*span).into(),
            }),
            Stmt::Return { value, span } => Some(Statement::Return {
                value: value.as_ref().map(|value| self.expression(value)),
                span: (*span).into(),
            }),
            Stmt::RepeatWhile {
                condition,
                body,
                span,
            } => Some(Statement::Loop {
                loop_kind: LoopKind::While {
                    condition: self.expression(condition),
                },
                body: self.block_contents(body),
                span: (*span).into(),
            }),
            Stmt::Expr(expression) => self.expression_statement(expression),
        }
    }

    fn expression_statement(&self, expression: &Expr) -> Option<Statement> {
        match self.expression(expression) {
            Expression::Call(call) => Some(Statement::Call {
                span: call.span,
                call,
            }),
            _ => None,
        }
    }

    fn expression(&self, expression: &Expr) -> Expression {
        match expression {
            Expr::Lit { value, span, .. } => Expression::Literal {
                value: match value {
                    Literal::Int(value) => LiteralValue::Integer(*value),
                    Literal::Float(value) => LiteralValue::Decimal(*value),
                    Literal::Text(value) => LiteralValue::Text(value.clone()),
                    Literal::Bool(value) => LiteralValue::Boolean(*value),
                    Literal::Unit => LiteralValue::Unit,
                },
                span: (*span).into(),
            },
            Expr::VarRef { id, span, .. } => Expression::Identifier {
                name: self.name_for(*id),
                span: (*span).into(),
            },
            Expr::Call {
                callee, args, span, ..
            } => Expression::Call(Call {
                callee: Box::new(self.expression(callee)),
                arguments: args
                    .iter()
                    .map(|argument| self.expression(argument))
                    .collect(),
                span: (*span).into(),
            }),
            Expr::BinOp {
                left,
                op,
                right,
                span,
                ..
            } => Expression::Binary {
                operation: self.binary_operation(*op),
                left: Box::new(self.expression(left)),
                right: Box::new(self.expression(right)),
                span: (*span).into(),
            },
            Expr::UnOp {
                op, operand, span, ..
            } => match self.unary_operation(*op) {
                Some(operation) => Expression::Unary {
                    operation,
                    operand: Box::new(self.expression(operand)),
                    span: (*span).into(),
                },
                None => Expression::Unsupported {
                    span: (*span).into(),
                },
            },
            Expr::List { elements, span, .. } => Expression::Collection {
                elements: elements
                    .iter()
                    .map(|element| self.expression(element))
                    .collect(),
                span: (*span).into(),
            },
            Expr::MacroCall {
                name, args, span, ..
            } => Expression::Call(Call {
                callee: Box::new(Expression::Identifier {
                    name: name.clone(),
                    span: (*span).into(),
                }),
                arguments: args
                    .iter()
                    .map(|argument| self.expression(argument))
                    .collect(),
                span: (*span).into(),
            }),
            Expr::FieldIndex { span, .. }
            | Expr::Index { span, .. }
            | Expr::StructInit { span, .. }
            | Expr::PostfixTry { span, .. }
            | Expr::Block(Block { span, .. }) => Expression::Unsupported {
                span: (*span).into(),
            },
        }
    }

    fn name_for(&self, id: VariableId) -> String {
        match self.symbols.get(id.0) {
            Some(SymbolKind::Variable(symbol)) => symbol.name.clone(),
            Some(SymbolKind::Function(symbol)) => symbol.name.clone(),
            Some(SymbolKind::Type(symbol)) => symbol.name.clone(),
            _ => "<unresolved>".to_owned(),
        }
    }

    fn type_for(&self, id: TypeId) -> ExportType {
        self.symbols
            .get_interned_type(id)
            .map(|ty| self.type_from_hir(ty))
            .unwrap_or(ExportType::Unknown)
    }

    fn type_from_hir(&self, ty: &Type) -> ExportType {
        match ty {
            Type::Int => ExportType::Integer,
            Type::Float => ExportType::Decimal,
            Type::Bool => ExportType::Boolean,
            Type::Text => ExportType::Text,
            Type::Unit => ExportType::Unit,
            Type::Reference(inner, mutable) => ExportType::Reference {
                mutable: *mutable,
                inner: Box::new(self.type_from_hir(inner)),
            },
            Type::Pointer(inner) => ExportType::Pointer {
                inner: Box::new(self.type_from_hir(inner)),
            },
            Type::List(inner) => ExportType::Collection {
                element: Box::new(self.type_from_hir(inner)),
            },
            Type::Dict(key, value) => ExportType::Map {
                key: Box::new(self.type_from_hir(key)),
                value: Box::new(self.type_from_hir(value)),
            },
            Type::Optional(inner) => ExportType::Optional {
                inner: Box::new(self.type_from_hir(inner)),
            },
            Type::Result(ok, err) => ExportType::Result {
                ok: Box::new(self.type_from_hir(ok)),
                err: Box::new(self.type_from_hir(err)),
            },
            Type::Function(parameters, returns) => ExportType::Function {
                parameters: parameters.iter().map(|ty| self.type_from_hir(ty)).collect(),
                returns: Box::new(self.type_from_hir(returns)),
            },
            Type::Named(name, arguments) => ExportType::Named {
                name: name.clone(),
                arguments: arguments.iter().map(|ty| self.type_from_hir(ty)).collect(),
            },
            Type::Var(_) => ExportType::Unknown,
        }
    }

    fn binary_operation(&self, operation: BinOp) -> BinaryOperation {
        match operation {
            BinOp::Add => BinaryOperation::Add,
            BinOp::Sub => BinaryOperation::Subtract,
            BinOp::Mul => BinaryOperation::Multiply,
            BinOp::Div => BinaryOperation::Divide,
            BinOp::Mod => BinaryOperation::Remainder,
            BinOp::Eq => BinaryOperation::Equal,
            BinOp::NotEq => BinaryOperation::NotEqual,
            BinOp::Lt | BinOp::IsBelow => BinaryOperation::LessThan,
            BinOp::Gt | BinOp::IsAbove | BinOp::Exceeds => BinaryOperation::GreaterThan,
            BinOp::LtEq => BinaryOperation::LessThanOrEqual,
            BinOp::GtEq => BinaryOperation::GreaterThanOrEqual,
            BinOp::And => BinaryOperation::And,
            BinOp::Or => BinaryOperation::Or,
        }
    }

    fn unary_operation(&self, operation: UnOp) -> Option<UnaryOperation> {
        match operation {
            UnOp::Neg => Some(UnaryOperation::Negate),
            UnOp::Not => Some(UnaryOperation::Not),
            UnOp::Deref | UnOp::Borrow(_) => None,
        }
    }
}

pub fn to_json(document: &ExportDocument) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vinglish_hir::symbol::{SymbolId, TypeId, VariableId};

    #[test]
    fn serializes_the_v1_contract_without_hir_fields() {
        let ty = TypeId(SymbolId(0));
        let variable = VariableId(SymbolId(1));
        let span = Span::new(0, 1);
        let module = HirModule {
            items: vec![Item::Function(FunctionDef {
                visibility: vinglish_parser::ast::Visibility::Private,
                is_foreign: false,
                id: vinglish_hir::symbol::FunctionId(SymbolId(2)),
                name: "calculate".to_owned(),
                params: vec![],
                ret_ty: ty,
                body: Expr::Block(Block {
                    stmts: vec![Stmt::Assign {
                        target: Expr::VarRef {
                            id: variable,
                            ty,
                            span,
                        },
                        op: AssignOp::AddAssign,
                        value: Expr::Lit {
                            value: Literal::Int(1),
                            ty,
                            span,
                        },
                        span,
                    }],
                    expr: None,
                    ty,
                    span,
                }),
                span,
            })],
        };

        let document =
            ExportBuilder::new(&SymbolTable::new()).document([(String::from("main"), &module)]);
        let json = to_json(&document).unwrap();

        assert!(json.contains("\"format\": \"vinglish.semantic-export\""));
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"kind\": \"mutation\""));
        assert!(!json.contains("VariableId"));
        assert!(!json.contains("TypeId"));
    }

    #[test]
    fn deserializes_its_own_versioned_document() {
        let document = ExportDocument::new(vec![Module {
            name: "main".to_owned(),
            functions: vec![],
            statements: vec![],
        }]);
        let json = to_json(&document).unwrap();
        assert_eq!(
            serde_json::from_str::<ExportDocument>(&json).unwrap(),
            document
        );
    }
}
