// KLIK Type System - Static type checker with inference

use klik_ast::types::Type;
use klik_ast::*;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("type mismatch: expected {expected}, found {found} at {span}")]
    Mismatch {
        expected: Type,
        found: Type,
        span: Span,
    },
    #[error("undefined type '{name}' at {span}")]
    UndefinedType { name: String, span: Span },
    #[error("cannot infer type at {span}")]
    CannotInfer { span: Span },
    #[error("type error: {message} at {span}")]
    General { message: String, span: Span },
}

/// Type environment for tracking bindings
#[derive(Debug, Clone)]
pub struct TypeEnv {
    scopes: Vec<HashMap<String, Type>>,
    type_defs: HashMap<String, TypeDef>,
    next_type_var: u64,
    substitutions: HashMap<u64, Type>,
}

#[derive(Debug, Clone)]
pub enum TypeDef {
    Struct {
        generic_params: Vec<String>,
        fields: Vec<(String, Type)>,
    },
    Enum {
        generic_params: Vec<String>,
        variants: Vec<(String, Vec<Type>)>,
    },
    Trait {
        methods: Vec<(String, Type)>,
    },
    Alias(Type),
}

impl TypeEnv {
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![HashMap::new()],
            type_defs: HashMap::new(),
            next_type_var: 0,
            substitutions: HashMap::new(),
        };
        env.register_builtins();
        env
    }

    fn register_builtins(&mut self) {
        // Register built-in functions
        self.bind(
            "print".into(),
            Type::Function(vec![Type::String], Box::new(Type::Void)),
        );
        self.bind(
            "println".into(),
            Type::Function(vec![Type::String], Box::new(Type::Void)),
        );
        self.bind(
            "assert".into(),
            Type::Function(vec![Type::Bool], Box::new(Type::Void)),
        );
        self.bind(
            "len".into(),
            Type::Function(vec![Type::String], Box::new(Type::Int)),
        );
        self.bind(
            "to_string".into(),
            Type::Function(vec![Type::Int], Box::new(Type::String)),
        );
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn bind(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    pub fn register_type_def(&mut self, name: String, def: TypeDef) {
        self.type_defs.insert(name, def);
    }

    pub fn lookup_type_def(&self, name: &str) -> Option<&TypeDef> {
        self.type_defs.get(name)
    }

    pub fn fresh_type_var(&mut self) -> Type {
        let id = self.next_type_var;
        self.next_type_var += 1;
        Type::TypeVar(id)
    }

    #[allow(clippy::result_large_err)]
    pub fn unify(&mut self, a: &Type, b: &Type, span: &Span) -> Result<Type, TypeError> {
        let a = self.resolve(a);
        let b = self.resolve(b);

        match (&a, &b) {
            _ if a == b => Ok(a),
            (Type::TypeVar(id), _) => {
                self.substitutions.insert(*id, b.clone());
                Ok(b)
            }
            (_, Type::TypeVar(id)) => {
                self.substitutions.insert(*id, a.clone());
                Ok(a)
            }
            (Type::Error, _) | (_, Type::Error) => Ok(Type::Error),
            (Type::Array(a_inner, a_size), Type::Array(b_inner, b_size)) => {
                let inner = self.unify(a_inner, b_inner, span)?;
                let size = match (a_size, b_size) {
                    (Some(s), _) | (_, Some(s)) => Some(*s),
                    _ => None,
                };
                Ok(Type::Array(Box::new(inner), size))
            }
            (Type::Optional(a_inner), Type::Optional(b_inner)) => {
                let inner = self.unify(a_inner, b_inner, span)?;
                Ok(Type::Optional(Box::new(inner)))
            }
            (Type::Tuple(a_elems), Type::Tuple(b_elems)) if a_elems.len() == b_elems.len() => {
                let mut elems = Vec::new();
                for (a, b) in a_elems.iter().zip(b_elems.iter()) {
                    elems.push(self.unify(a, b, span)?);
                }
                Ok(Type::Tuple(elems))
            }
            (Type::Function(a_params, a_ret), Type::Function(b_params, b_ret))
                if a_params.len() == b_params.len() =>
            {
                let mut params = Vec::new();
                for (a, b) in a_params.iter().zip(b_params.iter()) {
                    params.push(self.unify(a, b, span)?);
                }
                let ret = self.unify(a_ret, b_ret, span)?;
                Ok(Type::Function(params, Box::new(ret)))
            }
            // Numeric coercions
            (Type::Int, Type::Int64) | (Type::Int64, Type::Int) => Ok(Type::Int64),
            (Type::Float64, Type::Float32) | (Type::Float32, Type::Float64) => Ok(Type::Float64),
            _ => Err(TypeError::Mismatch {
                expected: a,
                found: b,
                span: span.clone(),
            }),
        }
    }

    pub fn resolve(&self, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(id) => {
                if let Some(resolved) = self.substitutions.get(id) {
                    self.resolve(resolved)
                } else {
                    ty.clone()
                }
            }
            _ => ty.clone(),
        }
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Type checker for the KLIK AST
pub struct TypeChecker {
    env: TypeEnv,
    errors: Vec<TypeError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            errors: Vec::new(),
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        // First pass: register all type definitions
        for module in &program.modules {
            self.register_module_types(module);
        }

        // Second pass: type check all items
        for module in &program.modules {
            self.check_module(module);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    fn register_module_types(&mut self, module: &Module) {
        for item in &module.items {
            match item {
                Item::Struct(s) => {
                    let fields: Vec<(String, Type)> = s
                        .fields
                        .iter()
                        .map(|f| (f.name.clone(), self.resolve_type_expr(&f.type_expr)))
                        .collect();
                    let generic_params: Vec<String> =
                        s.generic_params.iter().map(|g| g.name.clone()).collect();
                    self.env.register_type_def(
                        s.name.clone(),
                        TypeDef::Struct {
                            generic_params,
                            fields,
                        },
                    );
                }
                Item::Enum(e) => {
                    let variants: Vec<(String, Vec<Type>)> = e
                        .variants
                        .iter()
                        .map(|v| {
                            let fields: Vec<Type> =
                                v.fields.iter().map(|f| self.resolve_type_expr(f)).collect();
                            (v.name.clone(), fields)
                        })
                        .collect();
                    let generic_params: Vec<String> =
                        e.generic_params.iter().map(|g| g.name.clone()).collect();
                    self.env.register_type_def(
                        e.name.clone(),
                        TypeDef::Enum {
                            generic_params,
                            variants,
                        },
                    );
                }
                Item::TypeAlias(ta) => {
                    let resolved = self.resolve_type_expr(&ta.type_expr);
                    self.env
                        .register_type_def(ta.name.clone(), TypeDef::Alias(resolved));
                }
                _ => {}
            }
        }
    }

    fn check_module(&mut self, module: &Module) {
        self.predeclare_module_functions(module);
        for item in &module.items {
            self.check_item(item);
        }
    }

    fn predeclare_module_functions(&mut self, module: &Module) {
        for item in &module.items {
            if let Item::Function(f) = item {
                let params: Vec<Type> = f
                    .params
                    .iter()
                    .map(|p| self.resolve_type_expr(&p.type_expr))
                    .collect();
                let ret = f
                    .return_type
                    .as_ref()
                    .map(|t| self.resolve_type_expr(t))
                    .unwrap_or(Type::Void);
                self.env
                    .bind(f.name.clone(), Type::Function(params, Box::new(ret)));
            }
        }
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => self.check_function(f),
            Item::Impl(imp) => {
                // Bind `Self` type for the impl block
                let self_type = Type::Struct(imp.type_name.clone(), Vec::new());
                self.env.push_scope();
                self.env.bind("Self".to_string(), self_type.clone());
                for method in &imp.methods {
                    self.check_function(method);
                }
                self.env.pop_scope();
            }
            Item::Const(c) => {
                let val_ty = self.check_expr(&c.value);
                if let Some(ref type_expr) = c.type_expr {
                    let expected = self.resolve_type_expr(type_expr);
                    if let Err(e) = self.env.unify(&expected, &val_ty, &c.span) {
                        self.errors.push(e);
                    }
                }
            }
            Item::Test(t) => {
                self.env.push_scope();
                self.check_block(&t.body);
                self.env.pop_scope();
            }
            _ => {}
        }
    }

    fn check_function(&mut self, func: &Function) {
        self.env.push_scope();

        // Bind generic parameters
        for gp in &func.generic_params {
            self.env
                .bind(gp.name.clone(), Type::Generic(gp.name.clone()));
        }

        // Bind parameters
        let mut param_types = Vec::new();
        for param in &func.params {
            let ty = self.resolve_type_expr(&param.type_expr);
            self.env.bind(param.name.clone(), ty.clone());
            param_types.push(ty);
        }

        let return_type = func
            .return_type
            .as_ref()
            .map(|t| self.resolve_type_expr(t))
            .unwrap_or(Type::Void);

        // Register function in the parent scope to allow recursion
        let func_type = Type::Function(param_types, Box::new(return_type.clone()));
        self.env.bind(func.name.clone(), func_type);

        // Check body
        let body_type = self.check_block(&func.body);

        // Check return type
        if return_type != Type::Void {
            if let Err(e) = self.env.unify(&return_type, &body_type, &func.span) {
                self.errors.push(e);
            }
        }

        self.env.pop_scope();
    }

    fn check_block(&mut self, block: &Block) -> Type {
        let mut last_type = Type::Void;
        for stmt in &block.stmts {
            last_type = self.check_stmt(stmt);
        }
        last_type
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Type {
        match stmt {
            Stmt::Let(s) => {
                let val_type = if let Some(ref value) = s.value {
                    self.check_expr(value)
                } else {
                    self.env.fresh_type_var()
                };

                if let Some(ref type_expr) = s.type_expr {
                    let expected = self.resolve_type_expr(type_expr);
                    match self.env.unify(&expected, &val_type, &s.span) {
                        Ok(unified) => {
                            self.env.bind(s.name.clone(), unified);
                        }
                        Err(e) => {
                            self.errors.push(e);
                            self.env.bind(s.name.clone(), Type::Error);
                        }
                    }
                } else {
                    self.env.bind(s.name.clone(), val_type);
                }
                Type::Void
            }
            Stmt::Expr(expr) => self.check_expr(expr),
            Stmt::Return(ret) => {
                if let Some(ref value) = ret.value {
                    self.check_expr(value)
                } else {
                    Type::Void
                }
            }
            Stmt::While(w) => {
                let cond_ty = self.check_expr(&w.condition);
                if let Err(e) = self.env.unify(&Type::Bool, &cond_ty, &w.span) {
                    self.errors.push(e);
                }
                self.env.push_scope();
                self.check_block(&w.body);
                self.env.pop_scope();
                Type::Void
            }
            Stmt::For(f) => {
                let iter_ty = self.check_expr(&f.iterator);
                let elem_ty = match iter_ty {
                    Type::Array(inner, _) => *inner,
                    _ => self.env.fresh_type_var(),
                };
                self.env.push_scope();
                self.env.bind(f.variable.clone(), elem_ty);
                self.check_block(&f.body);
                self.env.pop_scope();
                Type::Void
            }
            Stmt::Assign(a) => {
                let target_ty = self.check_expr(&a.target);
                let val_ty = self.check_expr(&a.value);
                if let Err(e) = self.env.unify(&target_ty, &val_ty, &a.span) {
                    self.errors.push(e);
                }
                Type::Void
            }
            Stmt::Item(item) => {
                self.check_item(item);
                Type::Void
            }
            Stmt::Break(_) | Stmt::Continue(_) => Type::Never,
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(lit) => match &lit.kind {
                LiteralKind::Int(_) => Type::Int,
                LiteralKind::Float(_) => Type::Float64,
                LiteralKind::String(_) => Type::String,
                LiteralKind::Bool(_) => Type::Bool,
                LiteralKind::Char(_) => Type::Char,
                LiteralKind::None => Type::Optional(Box::new(self.env.fresh_type_var())),
            },
            Expr::Identifier(ident) => match self.env.lookup(&ident.name) {
                Some(ty) => ty.clone(),
                None => {
                    // Handle Enum::Variant paths
                    if ident.name.contains("::") {
                        let parts: Vec<&str> = ident.name.splitn(2, "::").collect();
                        if let Some(_td) = self.env.lookup_type_def(parts[0]) {
                            // Return the enum type for the base
                            return Type::Struct(parts[0].to_string(), Vec::new());
                        }
                    }
                    self.errors.push(TypeError::General {
                        message: format!("undefined variable '{}'", ident.name),
                        span: ident.span.clone(),
                    });
                    Type::Error
                }
            },
            Expr::Binary(bin) => {
                // Handle pipe operator specially: don't evaluate RHS normally
                if bin.op == BinaryOp::Pipe {
                    let left_ty = self.check_expr(&bin.left);
                    return self.check_pipe_expr(&left_ty, &bin.right, &bin.span);
                }

                let left_ty = self.check_expr(&bin.left);
                let right_ty = self.check_expr(&bin.right);

                match bin.op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => match self.env.unify(&left_ty, &right_ty, &bin.span) {
                        Ok(ty) if ty.is_numeric() => ty,
                        Ok(ty) if ty == Type::String && bin.op == BinaryOp::Add => Type::String,
                        Ok(ty) => {
                            self.errors.push(TypeError::General {
                                message: format!(
                                    "operator '{}' not supported for type {}",
                                    bin.op, ty
                                ),
                                span: bin.span.clone(),
                            });
                            Type::Error
                        }
                        Err(e) => {
                            self.errors.push(e);
                            Type::Error
                        }
                    },
                    BinaryOp::Eq
                    | BinaryOp::Neq
                    | BinaryOp::Lt
                    | BinaryOp::Gt
                    | BinaryOp::Lte
                    | BinaryOp::Gte => {
                        if let Err(e) = self.env.unify(&left_ty, &right_ty, &bin.span) {
                            self.errors.push(e);
                        }
                        Type::Bool
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if let Err(e) = self.env.unify(&Type::Bool, &left_ty, &bin.span) {
                            self.errors.push(e);
                        }
                        if let Err(e) = self.env.unify(&Type::Bool, &right_ty, &bin.span) {
                            self.errors.push(e);
                        }
                        Type::Bool
                    }
                    BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr => match self.env.unify(&left_ty, &right_ty, &bin.span) {
                        Ok(ty) if ty.is_integer() => ty,
                        Ok(_) => {
                            self.errors.push(TypeError::General {
                                message: "bitwise operations require integer types".into(),
                                span: bin.span.clone(),
                            });
                            Type::Error
                        }
                        Err(e) => {
                            self.errors.push(e);
                            Type::Error
                        }
                    },
                    BinaryOp::Pipe => {
                        // Handled above via check_pipe_expr
                        unreachable!()
                    }
                }
            }
            Expr::Unary(unary) => {
                let operand_ty = self.check_expr(&unary.operand);
                match unary.op {
                    UnaryOp::Neg => {
                        if !operand_ty.is_numeric() {
                            self.errors.push(TypeError::General {
                                message: format!("cannot negate type {}", operand_ty),
                                span: unary.span.clone(),
                            });
                        }
                        operand_ty
                    }
                    UnaryOp::Not => {
                        if let Err(e) = self.env.unify(&Type::Bool, &operand_ty, &unary.span) {
                            self.errors.push(e);
                        }
                        Type::Bool
                    }
                    UnaryOp::BitNot => {
                        if !operand_ty.is_integer() {
                            self.errors.push(TypeError::General {
                                message: "bitwise NOT requires integer type".into(),
                                span: unary.span.clone(),
                            });
                        }
                        operand_ty
                    }
                    UnaryOp::Ref => Type::Reference(Box::new(operand_ty), false),
                    UnaryOp::RefMut => Type::Reference(Box::new(operand_ty), true),
                    UnaryOp::Deref => match operand_ty {
                        Type::Reference(inner, _) => *inner,
                        _ => {
                            self.errors.push(TypeError::General {
                                message: format!("cannot dereference type {}", operand_ty),
                                span: unary.span.clone(),
                            });
                            Type::Error
                        }
                    },
                }
            }
            Expr::Call(call) => {
                let callee_ty = self.check_expr(&call.callee);
                let arg_types: Vec<Type> = call.args.iter().map(|a| self.check_expr(a)).collect();

                // Handle variadic builtins
                if let Expr::Identifier(ident) = &*call.callee {
                    match ident.name.as_str() {
                        "print" | "println" => return Type::Void,
                        "assert" => return Type::Void,
                        "len" => return Type::Int,
                        "to_string" => return Type::String,
                        "spawn" => return Type::Void,
                        "Some" => {
                            if arg_types.len() == 1 {
                                return Type::Optional(Box::new(arg_types[0].clone()));
                            }
                        }
                        "Ok" | "Err" => {
                            return self.env.fresh_type_var();
                        }
                        "map" | "filter" | "fold" | "reduce" | "for_each" | "collect" | "sum"
                        | "count" | "min" | "max" | "any" | "all" | "find" | "take" | "skip"
                        | "enumerate" | "zip" | "flat_map" => {
                            return self.env.fresh_type_var();
                        }
                        _ => {}
                    }
                }

                match callee_ty {
                    Type::Function(params, ret) => {
                        if params.len() != arg_types.len() {
                            self.errors.push(TypeError::General {
                                message: format!(
                                    "expected {} arguments, found {}",
                                    params.len(),
                                    arg_types.len()
                                ),
                                span: call.span.clone(),
                            });
                        } else {
                            for (param, arg) in params.iter().zip(arg_types.iter()) {
                                if let Err(e) = self.env.unify(param, arg, &call.span) {
                                    self.errors.push(e);
                                }
                            }
                        }
                        *ret
                    }
                    Type::Error => Type::Error,
                    _ => {
                        // Allow calling any expression (may be resolved later)
                        self.env.fresh_type_var()
                    }
                }
            }
            Expr::MethodCall(mc) => {
                let _receiver_ty = self.check_expr(&mc.receiver);
                let _arg_types: Vec<Type> = mc.args.iter().map(|a| self.check_expr(a)).collect();
                // Method resolution would go here
                self.env.fresh_type_var()
            }
            Expr::FieldAccess(fa) => {
                let obj_ty = self.check_expr(&fa.object);
                match &obj_ty {
                    Type::Struct(name, _) => {
                        if let Some(TypeDef::Struct { fields, .. }) = self.env.lookup_type_def(name)
                        {
                            let fields = fields.clone();
                            if let Some((_, ty)) = fields.iter().find(|(n, _)| n == &fa.field) {
                                ty.clone()
                            } else {
                                self.errors.push(TypeError::General {
                                    message: format!("no field '{}' on type {}", fa.field, name),
                                    span: fa.span.clone(),
                                });
                                Type::Error
                            }
                        } else {
                            self.env.fresh_type_var()
                        }
                    }
                    _ => self.env.fresh_type_var(),
                }
            }
            Expr::Index(idx) => {
                let obj_ty = self.check_expr(&idx.object);
                let idx_ty = self.check_expr(&idx.index);
                if let Err(e) = self.env.unify(&Type::Int, &idx_ty, &idx.span) {
                    self.errors.push(e);
                }
                match obj_ty {
                    Type::Array(inner, _) => *inner,
                    Type::String => Type::Char,
                    _ => self.env.fresh_type_var(),
                }
            }
            Expr::If(if_expr) => {
                let cond_ty = self.check_expr(&if_expr.condition);
                if let Err(e) = self.env.unify(&Type::Bool, &cond_ty, &if_expr.span) {
                    self.errors.push(e);
                }

                self.env.push_scope();
                let then_ty = self.check_block(&if_expr.then_block);
                self.env.pop_scope();

                if let Some(ref else_expr) = if_expr.else_block {
                    let else_ty = self.check_expr(else_expr);
                    match self.env.unify(&then_ty, &else_ty, &if_expr.span) {
                        Ok(ty) => ty,
                        Err(e) => {
                            self.errors.push(e);
                            Type::Error
                        }
                    }
                } else {
                    Type::Void
                }
            }
            Expr::Match(m) => {
                let _subject_ty = self.check_expr(&m.subject);
                let mut result_ty: Option<Type> = None;
                for arm in &m.arms {
                    let arm_ty = self.check_expr(&arm.body);
                    if let Some(ref prev_ty) = result_ty {
                        if let Err(e) = self.env.unify(prev_ty, &arm_ty, &arm.span) {
                            self.errors.push(e);
                        }
                    }
                    result_ty = Some(arm_ty);
                }
                result_ty.unwrap_or(Type::Void)
            }
            Expr::Block(block) => {
                self.env.push_scope();
                let ty = self.check_block(block);
                self.env.pop_scope();
                ty
            }
            Expr::Array(arr) => {
                if arr.elements.is_empty() {
                    Type::Array(Box::new(self.env.fresh_type_var()), Some(0))
                } else {
                    let first_ty = self.check_expr(&arr.elements[0]);
                    for elem in &arr.elements[1..] {
                        let elem_ty = self.check_expr(elem);
                        if let Err(e) = self.env.unify(&first_ty, &elem_ty, &arr.span) {
                            self.errors.push(e);
                        }
                    }
                    Type::Array(Box::new(first_ty), Some(arr.elements.len()))
                }
            }
            Expr::Tuple(tup) => {
                let types: Vec<Type> = tup.elements.iter().map(|e| self.check_expr(e)).collect();
                Type::Tuple(types)
            }
            Expr::StructInit(si) => {
                for (_, val) in &si.fields {
                    self.check_expr(val);
                }
                Type::Struct(si.name.clone(), Vec::new())
            }
            Expr::Lambda(l) => {
                self.env.push_scope();
                let param_types: Vec<Type> = l
                    .params
                    .iter()
                    .map(|p| {
                        let ty = self.resolve_type_expr(&p.type_expr);
                        self.env.bind(p.name.clone(), ty.clone());
                        ty
                    })
                    .collect();
                let body_ty = self.check_expr(&l.body);
                self.env.pop_scope();
                Type::Function(param_types, Box::new(body_ty))
            }
            Expr::Await(a) => self.check_expr(&a.expr),
            Expr::Range(_) => {
                Type::Array(Box::new(Type::Int), None) // Range produces int iterator
            }
            Expr::Cast(c) => {
                self.check_expr(&c.expr);
                self.resolve_type_expr(&c.type_expr)
            }
        }
    }

    /// Type-check a pipe expression: left |> right
    /// Handles iterator functions (map, filter, fold, sum, count, collect, etc.)
    fn check_pipe_expr(&mut self, left_ty: &Type, right: &Expr, span: &Span) -> Type {
        // Extract element type from array/collection
        let elem_ty = match left_ty {
            Type::Array(el, _) => *el.clone(),
            _ => left_ty.clone(),
        };

        // Check if RHS is a call to an iterator function
        if let Expr::Call(call) = right {
            if let Expr::Identifier(ident) = call.callee.as_ref() {
                match ident.name.as_str() {
                    "map" => {
                        // map(|x| expr) -> Array<result_type>
                        if let Some(arg) = call.args.first() {
                            let fn_ty = self.check_expr(arg);
                            let result_elem = match fn_ty {
                                Type::Function(_, ret) => *ret,
                                _ => elem_ty.clone(),
                            };
                            return Type::Array(Box::new(result_elem), None);
                        }
                        return Type::Array(Box::new(elem_ty), None);
                    }
                    "filter" => {
                        // filter(|x| bool) -> Array<same_elem>
                        if let Some(arg) = call.args.first() {
                            self.check_expr(arg);
                        }
                        return Type::Array(Box::new(elem_ty), None);
                    }
                    "sum" | "product" => {
                        // sum() / product() -> elem type (numeric)
                        return elem_ty;
                    }
                    "count" | "len" => {
                        // count() -> Int
                        return Type::Int;
                    }
                    "collect" => {
                        // collect() -> Array<elem>
                        return Type::Array(Box::new(elem_ty), None);
                    }
                    "fold" => {
                        // fold(init, |acc, x| expr) -> type of init
                        if let Some(init_arg) = call.args.first() {
                            let init_ty = self.check_expr(init_arg);
                            if let Some(fn_arg) = call.args.get(1) {
                                self.check_expr(fn_arg);
                            }
                            return init_ty;
                        }
                        return elem_ty;
                    }
                    "reduce" => {
                        // reduce(|acc, x| expr) -> elem_type
                        if let Some(arg) = call.args.first() {
                            self.check_expr(arg);
                        }
                        return elem_ty;
                    }
                    "for_each" => {
                        // for_each(|x| ...) -> Void
                        if let Some(arg) = call.args.first() {
                            self.check_expr(arg);
                        }
                        return Type::Void;
                    }
                    "any" | "all" => {
                        // any/all(|x| bool) -> Bool
                        if let Some(arg) = call.args.first() {
                            self.check_expr(arg);
                        }
                        return Type::Bool;
                    }
                    "find" => {
                        // find(|x| bool) -> Optional<elem>
                        if let Some(arg) = call.args.first() {
                            self.check_expr(arg);
                        }
                        return Type::Optional(Box::new(elem_ty));
                    }
                    "take" | "skip" | "zip" | "enumerate" | "flatten" | "flat_map" | "chain"
                    | "reverse" | "sort" | "sort_by" | "dedup" | "unique" | "chunks"
                    | "windows" => {
                        // Collection-preserving operations
                        for arg in &call.args {
                            self.check_expr(arg);
                        }
                        return Type::Array(Box::new(elem_ty), None);
                    }
                    "first" | "last" | "min" | "max" => {
                        // first/last/min/max -> Optional<elem>
                        return Type::Optional(Box::new(elem_ty));
                    }
                    "join" => {
                        // join(separator) -> String
                        for arg in &call.args {
                            self.check_expr(arg);
                        }
                        return Type::String;
                    }
                    _ => {} // fall through to normal function call check
                }
            }
        }

        // General case: RHS is a function expression
        let right_ty = self.check_expr(right);
        match &right_ty {
            Type::Function(params, ret) if !params.is_empty() => {
                if let Err(e) = self.env.unify(left_ty, &params[0], span) {
                    self.errors.push(e);
                }
                *ret.clone()
            }
            _ => {
                self.errors.push(TypeError::General {
                    message: "pipe operator requires a function or iterator method on the right"
                        .into(),
                    span: span.clone(),
                });
                Type::Error
            }
        }
    }

    pub fn resolve_type_expr(&self, type_expr: &TypeExpr) -> Type {
        match type_expr {
            TypeExpr::Named {
                name, generic_args, ..
            } => {
                let args: Vec<Type> = generic_args
                    .iter()
                    .map(|a| self.resolve_type_expr(a))
                    .collect();
                match name.as_str() {
                    "int" => Type::Int,
                    "i8" => Type::Int8,
                    "i16" => Type::Int16,
                    "i32" => Type::Int32,
                    "i64" => Type::Int64,
                    "uint" => Type::Uint,
                    "u8" => Type::Uint8,
                    "u16" => Type::Uint16,
                    "u32" => Type::Uint32,
                    "u64" => Type::Uint64,
                    "f32" => Type::Float32,
                    "f64" => Type::Float64,
                    "bool" => Type::Bool,
                    "char" => Type::Char,
                    "string" => Type::String,
                    "void" => Type::Void,
                    "never" => Type::Never,
                    "_" => Type::TypeVar(0), // infer
                    "Self" => {
                        // Resolve Self from the current scope
                        self.env
                            .lookup("Self")
                            .cloned()
                            .unwrap_or(Type::Generic("Self".to_string()))
                    }
                    _ => {
                        // Check if it's a known struct/enum
                        if self.env.lookup_type_def(name).is_some() {
                            Type::Struct(name.clone(), args)
                        } else {
                            Type::Generic(name.clone())
                        }
                    }
                }
            }
            TypeExpr::Array {
                element, size: _, ..
            } => Type::Array(Box::new(self.resolve_type_expr(element)), None),
            TypeExpr::Tuple { elements, .. } => {
                Type::Tuple(elements.iter().map(|e| self.resolve_type_expr(e)).collect())
            }
            TypeExpr::Function {
                params,
                return_type,
                ..
            } => Type::Function(
                params.iter().map(|p| self.resolve_type_expr(p)).collect(),
                Box::new(self.resolve_type_expr(return_type)),
            ),
            TypeExpr::Optional { inner, .. } => {
                Type::Optional(Box::new(self.resolve_type_expr(inner)))
            }
            TypeExpr::Reference { inner, mutable, .. } => {
                Type::Reference(Box::new(self.resolve_type_expr(inner)), *mutable)
            }
        }
    }

    pub fn env(&self) -> &TypeEnv {
        &self.env
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
