// KLIK Language - Abstract Syntax Tree
// Complete AST node definitions for the KLIK programming language

pub mod types;
pub mod visitor;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Source location metadata for error reporting
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub file: String,
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(file: impl Into<String>, start: Position, end: Position) -> Self {
        Self {
            file: file.into(),
            start,
            end,
        }
    }

    pub fn dummy() -> Self {
        Self {
            file: String::new(),
            start: Position {
                line: 0,
                column: 0,
                offset: 0,
            },
            end: Position {
                line: 0,
                column: 0,
                offset: 0,
            },
        }
    }

    pub fn merge(&self, other: &Span) -> Span {
        Span {
            file: self.file.clone(),
            start: self.start.clone(),
            end: other.end.clone(),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.file,
            self.start.line + 1,
            self.start.column + 1
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

/// A node with source location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

// ============================================================================
// Top-level program structure
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub modules: Vec<Module>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub items: Vec<Item>,
    pub span: Span,
}

/// Top-level items in a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    Function(Function),
    Struct(StructDef),
    Enum(EnumDef),
    Trait(TraitDef),
    Impl(ImplBlock),
    Import(ImportDecl),
    Const(ConstDecl),
    TypeAlias(TypeAlias),
    Module(Module),
    Test(TestDecl),
}

impl Item {
    pub fn span(&self) -> &Span {
        match self {
            Item::Function(f) => &f.span,
            Item::Struct(s) => &s.span,
            Item::Enum(e) => &e.span,
            Item::Trait(t) => &t.span,
            Item::Impl(i) => &i.span,
            Item::Import(i) => &i.span,
            Item::Const(c) => &c.span,
            Item::TypeAlias(t) => &t.span,
            Item::Module(m) => &m.span,
            Item::Test(t) => &t.span,
        }
    }
}

// ============================================================================
// Declarations
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub is_async: bool,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub type_expr: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<TypeExpr>,
    pub default: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub fields: Vec<FieldDef>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub type_expr: TypeExpr,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub variants: Vec<EnumVariant>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDef {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub methods: Vec<TraitMethod>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub default_body: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplBlock {
    pub type_name: String,
    pub generic_params: Vec<GenericParam>,
    pub trait_name: Option<String>,
    pub methods: Vec<Function>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub items: Option<Vec<String>>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstDecl {
    pub name: String,
    pub type_expr: Option<TypeExpr>,
    pub value: Expr,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAlias {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub type_expr: TypeExpr,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDecl {
    pub name: String,
    pub body: Block,
    pub span: Span,
}

// ============================================================================
// Type expressions
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeExpr {
    Named {
        name: String,
        generic_args: Vec<TypeExpr>,
        span: Span,
    },
    Array {
        element: Box<TypeExpr>,
        size: Option<Box<Expr>>,
        span: Span,
    },
    Tuple {
        elements: Vec<TypeExpr>,
        span: Span,
    },
    Function {
        params: Vec<TypeExpr>,
        return_type: Box<TypeExpr>,
        span: Span,
    },
    Optional {
        inner: Box<TypeExpr>,
        span: Span,
    },
    Reference {
        inner: Box<TypeExpr>,
        mutable: bool,
        span: Span,
    },
}

impl TypeExpr {
    pub fn span(&self) -> &Span {
        match self {
            TypeExpr::Named { span, .. } => span,
            TypeExpr::Array { span, .. } => span,
            TypeExpr::Tuple { span, .. } => span,
            TypeExpr::Function { span, .. } => span,
            TypeExpr::Optional { span, .. } => span,
            TypeExpr::Reference { span, .. } => span,
        }
    }
}

// ============================================================================
// Statements
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stmt {
    Let(LetStmt),
    Expr(Expr),
    Return(ReturnStmt),
    Break(Span),
    Continue(Span),
    While(WhileStmt),
    For(ForStmt),
    Assign(AssignStmt),
    Item(Item),
}

impl Stmt {
    pub fn span(&self) -> &Span {
        match self {
            Stmt::Let(s) => &s.span,
            Stmt::Expr(e) => e.span(),
            Stmt::Return(r) => &r.span,
            Stmt::Break(s) => s,
            Stmt::Continue(s) => s,
            Stmt::While(w) => &w.span,
            Stmt::For(f) => &f.span,
            Stmt::Assign(a) => &a.span,
            Stmt::Item(i) => i.span(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetStmt {
    pub name: String,
    pub type_expr: Option<TypeExpr>,
    pub value: Option<Expr>,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForStmt {
    pub variable: String,
    pub iterator: Expr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignStmt {
    pub target: Expr,
    pub value: Expr,
    pub op: Option<BinaryOp>,
    pub span: Span,
}

// ============================================================================
// Expressions
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Literal(Literal),
    Identifier(Identifier),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    MethodCall(MethodCallExpr),
    FieldAccess(FieldAccessExpr),
    Index(IndexExpr),
    If(IfExpr),
    Match(MatchExpr),
    Block(Block),
    Array(ArrayExpr),
    Tuple(TupleExpr),
    StructInit(StructInitExpr),
    Lambda(LambdaExpr),
    Await(AwaitExpr),
    Range(RangeExpr),
    Cast(CastExpr),
}

impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            Expr::Literal(l) => &l.span,
            Expr::Identifier(i) => &i.span,
            Expr::Binary(b) => &b.span,
            Expr::Unary(u) => &u.span,
            Expr::Call(c) => &c.span,
            Expr::MethodCall(m) => &m.span,
            Expr::FieldAccess(f) => &f.span,
            Expr::Index(i) => &i.span,
            Expr::If(i) => &i.span,
            Expr::Match(m) => &m.span,
            Expr::Block(b) => &b.span,
            Expr::Array(a) => &a.span,
            Expr::Tuple(t) => &t.span,
            Expr::StructInit(s) => &s.span,
            Expr::Lambda(l) => &l.span,
            Expr::Await(a) => &a.span,
            Expr::Range(r) => &r.span,
            Expr::Cast(c) => &c.span,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Literal {
    pub kind: LiteralKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralKind {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Char(char),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub op: BinaryOp,
    pub right: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Pipe,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mod => write!(f, "%"),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Neq => write!(f, "!="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Lte => write!(f, "<="),
            BinaryOp::Gte => write!(f, ">="),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
            BinaryOp::BitAnd => write!(f, "&"),
            BinaryOp::BitOr => write!(f, "|"),
            BinaryOp::BitXor => write!(f, "^"),
            BinaryOp::Shl => write!(f, "<<"),
            BinaryOp::Shr => write!(f, ">>"),
            BinaryOp::Pipe => write!(f, "|>"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    Ref,
    RefMut,
    Deref,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
    pub generic_args: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodCallExpr {
    pub receiver: Box<Expr>,
    pub method: String,
    pub args: Vec<Expr>,
    pub generic_args: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldAccessExpr {
    pub object: Box<Expr>,
    pub field: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexExpr {
    pub object: Box<Expr>,
    pub index: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfExpr {
    pub condition: Box<Expr>,
    pub then_block: Block,
    pub else_block: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchExpr {
    pub subject: Box<Expr>,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Pattern {
    Literal(Literal),
    Identifier(String, Span),
    Tuple(Vec<Pattern>, Span),
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
        span: Span,
    },
    Enum {
        name: String,
        variant: String,
        fields: Vec<Pattern>,
        span: Span,
    },
    Wildcard(Span),
    Or(Vec<Pattern>, Span),
}

impl Pattern {
    pub fn span(&self) -> &Span {
        match self {
            Pattern::Literal(l) => &l.span,
            Pattern::Identifier(_, s) => s,
            Pattern::Tuple(_, s) => s,
            Pattern::Struct { span, .. } => span,
            Pattern::Enum { span, .. } => span,
            Pattern::Wildcard(s) => s,
            Pattern::Or(_, s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrayExpr {
    pub elements: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TupleExpr {
    pub elements: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructInitExpr {
    pub name: String,
    pub fields: Vec<(String, Expr)>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaExpr {
    pub params: Vec<Param>,
    pub body: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitExpr {
    pub expr: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeExpr {
    pub start: Option<Box<Expr>>,
    pub end: Option<Box<Expr>>,
    pub inclusive: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastExpr {
    pub expr: Box<Expr>,
    pub type_expr: TypeExpr,
    pub span: Span,
}
