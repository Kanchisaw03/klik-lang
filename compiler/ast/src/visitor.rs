// KLIK AST Visitor pattern for tree traversal

use crate::*;

/// Visitor trait for traversing the AST
pub trait Visitor: Sized {
    fn visit_program(&mut self, program: &Program) {
        walk_program(self, program);
    }
    fn visit_module(&mut self, module: &Module) {
        walk_module(self, module);
    }
    fn visit_item(&mut self, item: &Item) {
        walk_item(self, item);
    }
    fn visit_function(&mut self, func: &Function) {
        walk_function(self, func);
    }
    fn visit_struct_def(&mut self, def: &StructDef) {
        let _ = def;
    }
    fn visit_enum_def(&mut self, def: &EnumDef) {
        let _ = def;
    }
    fn visit_trait_def(&mut self, def: &TraitDef) {
        let _ = def;
    }
    fn visit_impl_block(&mut self, block: &ImplBlock) {
        walk_impl_block(self, block);
    }
    fn visit_import(&mut self, import: &ImportDecl) {
        let _ = import;
    }
    fn visit_const(&mut self, decl: &ConstDecl) {
        self.visit_expr(&decl.value);
    }
    fn visit_block(&mut self, block: &Block) {
        walk_block(self, block);
    }
    fn visit_stmt(&mut self, stmt: &Stmt) {
        walk_stmt(self, stmt);
    }
    fn visit_expr(&mut self, expr: &Expr) {
        walk_expr(self, expr);
    }
    fn visit_pattern(&mut self, pattern: &Pattern) {
        let _ = pattern;
    }
    fn visit_type_expr(&mut self, type_expr: &TypeExpr) {
        let _ = type_expr;
    }
}

pub fn walk_program<V: Visitor>(visitor: &mut V, program: &Program) {
    for module in &program.modules {
        visitor.visit_module(module);
    }
}

pub fn walk_module<V: Visitor>(visitor: &mut V, module: &Module) {
    for item in &module.items {
        visitor.visit_item(item);
    }
}

pub fn walk_item<V: Visitor>(visitor: &mut V, item: &Item) {
    match item {
        Item::Function(f) => visitor.visit_function(f),
        Item::Struct(s) => visitor.visit_struct_def(s),
        Item::Enum(e) => visitor.visit_enum_def(e),
        Item::Trait(t) => visitor.visit_trait_def(t),
        Item::Impl(i) => visitor.visit_impl_block(i),
        Item::Import(i) => visitor.visit_import(i),
        Item::Const(c) => visitor.visit_const(c),
        Item::TypeAlias(_) => {}
        Item::Module(m) => visitor.visit_module(m),
        Item::Test(t) => visitor.visit_block(&t.body),
    }
}

pub fn walk_function<V: Visitor>(visitor: &mut V, func: &Function) {
    visitor.visit_block(&func.body);
}

pub fn walk_impl_block<V: Visitor>(visitor: &mut V, block: &ImplBlock) {
    for method in &block.methods {
        visitor.visit_function(method);
    }
}

pub fn walk_block<V: Visitor>(visitor: &mut V, block: &Block) {
    for stmt in &block.stmts {
        visitor.visit_stmt(stmt);
    }
}

pub fn walk_stmt<V: Visitor>(visitor: &mut V, stmt: &Stmt) {
    match stmt {
        Stmt::Let(s) => {
            if let Some(val) = &s.value {
                visitor.visit_expr(val);
            }
        }
        Stmt::Expr(e) => visitor.visit_expr(e),
        Stmt::Return(r) => {
            if let Some(val) = &r.value {
                visitor.visit_expr(val);
            }
        }
        Stmt::While(w) => {
            visitor.visit_expr(&w.condition);
            visitor.visit_block(&w.body);
        }
        Stmt::For(f) => {
            visitor.visit_expr(&f.iterator);
            visitor.visit_block(&f.body);
        }
        Stmt::Assign(a) => {
            visitor.visit_expr(&a.target);
            visitor.visit_expr(&a.value);
        }
        Stmt::Item(i) => visitor.visit_item(i),
        Stmt::Break(_) | Stmt::Continue(_) => {}
    }
}

pub fn walk_expr<V: Visitor>(visitor: &mut V, expr: &Expr) {
    match expr {
        Expr::Binary(b) => {
            visitor.visit_expr(&b.left);
            visitor.visit_expr(&b.right);
        }
        Expr::Unary(u) => {
            visitor.visit_expr(&u.operand);
        }
        Expr::Call(c) => {
            visitor.visit_expr(&c.callee);
            for arg in &c.args {
                visitor.visit_expr(arg);
            }
        }
        Expr::MethodCall(m) => {
            visitor.visit_expr(&m.receiver);
            for arg in &m.args {
                visitor.visit_expr(arg);
            }
        }
        Expr::FieldAccess(f) => {
            visitor.visit_expr(&f.object);
        }
        Expr::Index(i) => {
            visitor.visit_expr(&i.object);
            visitor.visit_expr(&i.index);
        }
        Expr::If(i) => {
            visitor.visit_expr(&i.condition);
            visitor.visit_block(&i.then_block);
            if let Some(else_block) = &i.else_block {
                visitor.visit_expr(else_block);
            }
        }
        Expr::Match(m) => {
            visitor.visit_expr(&m.subject);
            for arm in &m.arms {
                visitor.visit_pattern(&arm.pattern);
                visitor.visit_expr(&arm.body);
            }
        }
        Expr::Block(b) => {
            visitor.visit_block(b);
        }
        Expr::Array(a) => {
            for elem in &a.elements {
                visitor.visit_expr(elem);
            }
        }
        Expr::Tuple(t) => {
            for elem in &t.elements {
                visitor.visit_expr(elem);
            }
        }
        Expr::StructInit(s) => {
            for (_, val) in &s.fields {
                visitor.visit_expr(val);
            }
        }
        Expr::Lambda(l) => {
            visitor.visit_expr(&l.body);
        }
        Expr::Await(a) => {
            visitor.visit_expr(&a.expr);
        }
        Expr::Range(r) => {
            if let Some(start) = &r.start {
                visitor.visit_expr(start);
            }
            if let Some(end) = &r.end {
                visitor.visit_expr(end);
            }
        }
        Expr::Cast(c) => {
            visitor.visit_expr(&c.expr);
        }
        Expr::Literal(_) | Expr::Identifier(_) => {}
    }
}
