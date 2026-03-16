// KLIK Semantic Analysis
// Symbol resolution, scope tracking, import resolution, lifetime analysis

use klik_ast::visitor::Visitor;
use klik_ast::*;
use klik_type_system::{TypeChecker, TypeError};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SemanticError {
    #[error("undefined symbol '{name}' at {span}")]
    UndefinedSymbol { name: String, span: Span },
    #[error("duplicate definition '{name}' at {span}")]
    DuplicateDefinition { name: String, span: Span },
    #[error("cannot mutate immutable variable '{name}' at {span}")]
    ImmutableMutation { name: String, span: Span },
    #[error("break/continue outside of loop at {span}")]
    BreakOutsideLoop { span: Span },
    #[error("return outside of function at {span}")]
    ReturnOutsideFunction { span: Span },
    #[error("semantic error: {message} at {span}")]
    General { message: String, span: Span },
    #[error("type error: {0}")]
    TypeError(#[from] TypeError),
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub mutable: bool,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Variable,
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Constant,
    Parameter,
    TypeAlias,
}

/// Scope for tracking symbols
#[derive(Debug)]
struct Scope {
    symbols: HashMap<String, Symbol>,
    kind: ScopeKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ScopeKind {
    Global,
    Function,
    Block,
    Loop,
    Module,
}

/// Semantic analyzer performs name resolution and semantic checks
pub struct SemanticAnalyzer {
    scopes: Vec<Scope>,
    errors: Vec<SemanticError>,
    in_function: bool,
    loop_depth: usize,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            scopes: Vec::new(),
            errors: Vec::new(),
            in_function: false,
            loop_depth: 0,
        };
        analyzer.push_scope(ScopeKind::Global);
        // Register built-in symbols
        analyzer.define_symbol("print", SymbolKind::Function, Span::dummy(), false);
        analyzer.define_symbol("println", SymbolKind::Function, Span::dummy(), false);
        analyzer.define_symbol("assert", SymbolKind::Function, Span::dummy(), false);
        analyzer.define_symbol("len", SymbolKind::Function, Span::dummy(), false);
        analyzer.define_symbol("to_string", SymbolKind::Function, Span::dummy(), false);
        analyzer.define_symbol("spawn", SymbolKind::Function, Span::dummy(), false);
        analyzer
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        // First pass: collect all top-level declarations
        for module in &program.modules {
            self.collect_declarations(module);
        }

        // Second pass: resolve all symbols
        for module in &program.modules {
            self.visit_module(module);
        }

        // Third pass: type checking
        let mut type_checker = TypeChecker::new();
        if let Err(type_errors) = type_checker.check_program(program) {
            for te in type_errors {
                self.errors.push(SemanticError::TypeError(te));
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    fn collect_declarations(&mut self, module: &Module) {
        for item in &module.items {
            match item {
                Item::Function(f) => {
                    self.define_symbol(&f.name, SymbolKind::Function, f.span.clone(), false);
                }
                Item::Struct(s) => {
                    self.define_symbol(&s.name, SymbolKind::Struct, s.span.clone(), false);
                }
                Item::Enum(e) => {
                    self.define_symbol(&e.name, SymbolKind::Enum, e.span.clone(), false);
                }
                Item::Trait(t) => {
                    self.define_symbol(&t.name, SymbolKind::Trait, t.span.clone(), false);
                }
                Item::Const(c) => {
                    self.define_symbol(&c.name, SymbolKind::Constant, c.span.clone(), false);
                }
                Item::TypeAlias(ta) => {
                    self.define_symbol(&ta.name, SymbolKind::TypeAlias, ta.span.clone(), false);
                }
                Item::Module(m) => {
                    self.define_symbol(&m.name, SymbolKind::Module, m.span.clone(), false);
                }
                _ => {}
            }
        }
    }

    fn push_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope {
            symbols: HashMap::new(),
            kind,
        });
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_symbol(&mut self, name: &str, kind: SymbolKind, span: Span, mutable: bool) {
        let symbol = Symbol {
            name: name.to_string(),
            kind,
            span: span.clone(),
            mutable,
        };
        if let Some(scope) = self.scopes.last_mut() {
            if scope.symbols.contains_key(name) && !matches!(scope.kind, ScopeKind::Global) {
                self.errors.push(SemanticError::DuplicateDefinition {
                    name: name.to_string(),
                    span,
                });
            } else {
                scope.symbols.insert(name.to_string(), symbol);
            }
        }
    }

    fn resolve_symbol(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.symbols.get(name) {
                return Some(sym);
            }
        }
        None
    }

    fn is_in_loop(&self) -> bool {
        self.loop_depth > 0
    }

    fn check_expr_symbols(&mut self, expr: &Expr) {
        match expr {
            Expr::Identifier(ident) => {
                if self.resolve_symbol(&ident.name).is_none() {
                    // Check if it's a path like Enum::Variant
                    if let Some(base) = ident.name.split("::").next() {
                        if self.resolve_symbol(base).is_some() {
                            return; // Base type exists, variant reference is OK
                        }
                    }
                    self.errors.push(SemanticError::UndefinedSymbol {
                        name: ident.name.clone(),
                        span: ident.span.clone(),
                    });
                }
            }
            Expr::Binary(bin) => {
                self.check_expr_symbols(&bin.left);
                if bin.op == BinaryOp::Pipe {
                    // For pipe operator, the RHS may be an iterator function call
                    // (map, filter, fold, etc.) that doesn't exist as a standalone symbol
                    self.check_pipe_rhs(&bin.right);
                } else {
                    self.check_expr_symbols(&bin.right);
                }
            }
            Expr::Unary(unary) => {
                self.check_expr_symbols(&unary.operand);
            }
            Expr::Call(call) => {
                self.check_expr_symbols(&call.callee);
                for arg in &call.args {
                    self.check_expr_symbols(arg);
                }
            }
            Expr::MethodCall(mc) => {
                self.check_expr_symbols(&mc.receiver);
                for arg in &mc.args {
                    self.check_expr_symbols(arg);
                }
            }
            Expr::FieldAccess(fa) => {
                self.check_expr_symbols(&fa.object);
            }
            Expr::Index(idx) => {
                self.check_expr_symbols(&idx.object);
                self.check_expr_symbols(&idx.index);
            }
            Expr::If(if_expr) => {
                self.check_expr_symbols(&if_expr.condition);
                self.push_scope(ScopeKind::Block);
                for stmt in &if_expr.then_block.stmts {
                    self.check_stmt_symbols(stmt);
                }
                self.pop_scope();
                if let Some(ref else_expr) = if_expr.else_block {
                    self.check_expr_symbols(else_expr);
                }
            }
            Expr::Match(m) => {
                self.check_expr_symbols(&m.subject);
                for arm in &m.arms {
                    self.push_scope(ScopeKind::Block);
                    self.bind_pattern(&arm.pattern);
                    self.check_expr_symbols(&arm.body);
                    self.pop_scope();
                }
            }
            Expr::Block(block) => {
                self.push_scope(ScopeKind::Block);
                for stmt in &block.stmts {
                    self.check_stmt_symbols(stmt);
                }
                self.pop_scope();
            }
            Expr::Array(arr) => {
                for elem in &arr.elements {
                    self.check_expr_symbols(elem);
                }
            }
            Expr::Tuple(tup) => {
                for elem in &tup.elements {
                    self.check_expr_symbols(elem);
                }
            }
            Expr::StructInit(si) => {
                for (_, val) in &si.fields {
                    self.check_expr_symbols(val);
                }
            }
            Expr::Lambda(l) => {
                self.push_scope(ScopeKind::Function);
                for p in &l.params {
                    self.define_symbol(&p.name, SymbolKind::Parameter, p.span.clone(), false);
                }
                self.check_expr_symbols(&l.body);
                self.pop_scope();
            }
            Expr::Await(a) => {
                self.check_expr_symbols(&a.expr);
            }
            Expr::Range(r) => {
                if let Some(ref start) = r.start {
                    self.check_expr_symbols(start);
                }
                if let Some(ref end) = r.end {
                    self.check_expr_symbols(end);
                }
            }
            Expr::Cast(c) => {
                self.check_expr_symbols(&c.expr);
            }
            Expr::Literal(_) => {}
        }
    }

    /// Check RHS of a pipe expression, allowing known iterator function names
    fn check_pipe_rhs(&mut self, expr: &Expr) {
        match expr {
            Expr::Call(call) => {
                // Check if callee is a known iterator function
                let is_iter_fn = if let Expr::Identifier(ident) = call.callee.as_ref() {
                    matches!(
                        ident.name.as_str(),
                        "map"
                            | "filter"
                            | "fold"
                            | "reduce"
                            | "sum"
                            | "product"
                            | "count"
                            | "collect"
                            | "for_each"
                            | "any"
                            | "all"
                            | "find"
                            | "take"
                            | "skip"
                            | "zip"
                            | "enumerate"
                            | "flatten"
                            | "flat_map"
                            | "chain"
                            | "reverse"
                            | "sort"
                            | "sort_by"
                            | "dedup"
                            | "unique"
                            | "chunks"
                            | "windows"
                            | "first"
                            | "last"
                            | "min"
                            | "max"
                            | "join"
                    )
                } else {
                    false
                };

                if !is_iter_fn {
                    self.check_expr_symbols(&call.callee);
                }
                // Always check args (lambdas, etc.)
                for arg in &call.args {
                    self.check_expr_symbols(arg);
                }
            }
            Expr::Binary(bin) if bin.op == BinaryOp::Pipe => {
                // Chained pipes: a |> f() |> g()
                self.check_pipe_rhs(&bin.left);
                self.check_pipe_rhs(&bin.right);
            }
            _ => self.check_expr_symbols(expr),
        }
    }

    fn check_stmt_symbols(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(s) => {
                if let Some(ref value) = s.value {
                    self.check_expr_symbols(value);
                }
                self.define_symbol(&s.name, SymbolKind::Variable, s.span.clone(), s.mutable);
            }
            Stmt::Expr(e) => self.check_expr_symbols(e),
            Stmt::Return(r) => {
                if !self.in_function {
                    self.errors.push(SemanticError::ReturnOutsideFunction {
                        span: r.span.clone(),
                    });
                }
                if let Some(ref val) = r.value {
                    self.check_expr_symbols(val);
                }
            }
            Stmt::Break(span) => {
                if !self.is_in_loop() {
                    self.errors
                        .push(SemanticError::BreakOutsideLoop { span: span.clone() });
                }
            }
            Stmt::Continue(span) => {
                if !self.is_in_loop() {
                    self.errors
                        .push(SemanticError::BreakOutsideLoop { span: span.clone() });
                }
            }
            Stmt::While(w) => {
                self.check_expr_symbols(&w.condition);
                self.loop_depth += 1;
                self.push_scope(ScopeKind::Loop);
                for s in &w.body.stmts {
                    self.check_stmt_symbols(s);
                }
                self.pop_scope();
                self.loop_depth -= 1;
            }
            Stmt::For(f) => {
                self.check_expr_symbols(&f.iterator);
                self.loop_depth += 1;
                self.push_scope(ScopeKind::Loop);
                self.define_symbol(&f.variable, SymbolKind::Variable, f.span.clone(), false);
                for s in &f.body.stmts {
                    self.check_stmt_symbols(s);
                }
                self.pop_scope();
                self.loop_depth -= 1;
            }
            Stmt::Assign(a) => {
                self.check_expr_symbols(&a.target);
                self.check_expr_symbols(&a.value);
                // Check mutability
                if let Expr::Identifier(ident) = &a.target {
                    if let Some(sym) = self.resolve_symbol(&ident.name) {
                        if !sym.mutable && matches!(sym.kind, SymbolKind::Variable) {
                            self.errors.push(SemanticError::ImmutableMutation {
                                name: ident.name.clone(),
                                span: a.span.clone(),
                            });
                        }
                    }
                }
            }
            Stmt::Item(item) => {
                if let Item::Function(f) = item {
                    self.define_symbol(&f.name, SymbolKind::Function, f.span.clone(), false);
                    self.visit_function(f);
                }
            }
        }
    }

    fn bind_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Identifier(name, span) => {
                self.define_symbol(name, SymbolKind::Variable, span.clone(), false);
            }
            Pattern::Tuple(patterns, _) => {
                for p in patterns {
                    self.bind_pattern(p);
                }
            }
            Pattern::Enum { fields, .. } => {
                for f in fields {
                    self.bind_pattern(f);
                }
            }
            Pattern::Struct { fields, .. } => {
                for (_, p) in fields {
                    self.bind_pattern(p);
                }
            }
            Pattern::Or(patterns, _) => {
                for p in patterns {
                    self.bind_pattern(p);
                }
            }
            Pattern::Literal(_) | Pattern::Wildcard(_) => {}
        }
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor for SemanticAnalyzer {
    fn visit_function(&mut self, func: &Function) {
        self.push_scope(ScopeKind::Function);
        let was_in_function = self.in_function;
        self.in_function = true;

        for param in &func.params {
            self.define_symbol(
                &param.name,
                SymbolKind::Parameter,
                param.span.clone(),
                false,
            );
        }

        for stmt in &func.body.stmts {
            self.check_stmt_symbols(stmt);
        }

        self.in_function = was_in_function;
        self.pop_scope();
    }

    fn visit_module(&mut self, module: &Module) {
        self.push_scope(ScopeKind::Module);
        self.collect_declarations(module);

        for item in &module.items {
            match item {
                Item::Function(f) => self.visit_function(f),
                Item::Impl(imp) => {
                    for method in &imp.methods {
                        self.visit_function(method);
                    }
                }
                Item::Test(t) => {
                    self.push_scope(ScopeKind::Function);
                    self.in_function = true;
                    for stmt in &t.body.stmts {
                        self.check_stmt_symbols(stmt);
                    }
                    self.in_function = false;
                    self.pop_scope();
                }
                _ => {}
            }
        }

        self.pop_scope();
    }
}

/// Convenience function
pub fn analyze(program: &Program) -> Result<(), Vec<SemanticError>> {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(program)
}
