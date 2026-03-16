// KLIK Formatter - Pretty-prints KLIK AST back to source code

use klik_ast::*;

const INDENT: &str = "    ";

/// Format an entire program
pub fn format_program(program: &Program) -> String {
    let mut f = Formatter::new();
    f.format_program(program);
    f.output
}

struct Formatter {
    output: String,
    indent_level: usize,
}

impl Formatter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
        }
    }

    fn indent(&mut self) {
        for _ in 0..self.indent_level {
            self.output.push_str(INDENT);
        }
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }

    fn format_program(&mut self, program: &Program) {
        for (i, module) in program.modules.iter().enumerate() {
            if i > 0 {
                self.newline();
            }
            self.format_module_items(module);
        }
        if !self.output.ends_with('\n') {
            self.newline();
        }
    }

    fn format_module_items(&mut self, module: &Module) {
        let mut first = true;
        for item in &module.items {
            if !first {
                self.newline();
            }
            self.format_item(item);
            first = false;
        }
    }

    fn format_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => self.format_function(f),
            Item::Struct(s) => self.format_struct(s),
            Item::Enum(e) => self.format_enum(e),
            Item::Trait(t) => self.format_trait(t),
            Item::Impl(i) => self.format_impl(i),
            Item::Import(i) => self.format_import(i),
            Item::Const(c) => self.format_const(c),
            Item::TypeAlias(t) => self.format_type_alias(t),
            Item::Module(m) => self.format_module(m),
            Item::Test(t) => self.format_test(t),
        }
    }

    fn format_function(&mut self, func: &Function) {
        self.indent();
        if func.is_pub {
            self.output.push_str("pub ");
        }
        if func.is_async {
            self.output.push_str("async ");
        }
        self.output.push_str("fn ");
        self.output.push_str(&func.name);
        self.format_generic_params(&func.generic_params);
        self.output.push('(');
        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.format_param(param);
        }
        self.output.push(')');

        if let Some(ret) = &func.return_type {
            self.output.push_str(" -> ");
            self.format_type_expr(ret);
        }

        self.output.push_str(" {");
        self.newline();
        self.indent_level += 1;
        self.format_block(&func.body);
        self.indent_level -= 1;
        self.indent();
        self.output.push('}');
        self.newline();
    }

    fn format_param(&mut self, param: &Param) {
        self.output.push_str(&param.name);
        self.output.push_str(": ");
        self.format_type_expr(&param.type_expr);
        if let Some(default) = &param.default {
            self.output.push_str(" = ");
            self.format_expr(default);
        }
    }

    fn format_generic_params(&mut self, params: &[GenericParam]) {
        if params.is_empty() {
            return;
        }
        self.output.push('<');
        for (i, gp) in params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(&gp.name);
            if !gp.bounds.is_empty() {
                self.output.push_str(": ");
                for (j, bound) in gp.bounds.iter().enumerate() {
                    if j > 0 {
                        self.output.push_str(" + ");
                    }
                    self.format_type_expr(bound);
                }
            }
            if let Some(default) = &gp.default {
                self.output.push_str(" = ");
                self.format_type_expr(default);
            }
        }
        self.output.push('>');
    }

    fn format_struct(&mut self, s: &StructDef) {
        self.indent();
        if s.is_pub {
            self.output.push_str("pub ");
        }
        self.output.push_str("struct ");
        self.output.push_str(&s.name);
        self.format_generic_params(&s.generic_params);
        self.output.push_str(" {");
        self.newline();
        self.indent_level += 1;

        for field in &s.fields {
            self.indent();
            if field.is_pub {
                self.output.push_str("pub ");
            }
            self.output.push_str(&field.name);
            self.output.push_str(": ");
            self.format_type_expr(&field.type_expr);
            self.output.push(',');
            self.newline();
        }

        self.indent_level -= 1;
        self.indent();
        self.output.push('}');
        self.newline();
    }

    fn format_enum(&mut self, e: &EnumDef) {
        self.indent();
        if e.is_pub {
            self.output.push_str("pub ");
        }
        self.output.push_str("enum ");
        self.output.push_str(&e.name);
        self.format_generic_params(&e.generic_params);
        self.output.push_str(" {");
        self.newline();
        self.indent_level += 1;

        for variant in &e.variants {
            self.indent();
            self.output.push_str(&variant.name);
            if !variant.fields.is_empty() {
                self.output.push('(');
                for (i, field) in variant.fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_type_expr(field);
                }
                self.output.push(')');
            }
            self.output.push(',');
            self.newline();
        }

        self.indent_level -= 1;
        self.indent();
        self.output.push('}');
        self.newline();
    }

    fn format_trait(&mut self, t: &TraitDef) {
        self.indent();
        if t.is_pub {
            self.output.push_str("pub ");
        }
        self.output.push_str("trait ");
        self.output.push_str(&t.name);
        self.format_generic_params(&t.generic_params);
        self.output.push_str(" {");
        self.newline();
        self.indent_level += 1;

        for method in &t.methods {
            self.format_trait_method(method);
        }

        self.indent_level -= 1;
        self.indent();
        self.output.push('}');
        self.newline();
    }

    fn format_trait_method(&mut self, method: &TraitMethod) {
        self.indent();
        self.output.push_str("fn ");
        self.output.push_str(&method.name);
        self.output.push('(');
        for (i, param) in method.params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.format_param(param);
        }
        self.output.push(')');

        if let Some(ret) = &method.return_type {
            self.output.push_str(" -> ");
            self.format_type_expr(ret);
        }

        if let Some(body) = &method.default_body {
            self.output.push_str(" {");
            self.newline();
            self.indent_level += 1;
            self.format_block(body);
            self.indent_level -= 1;
            self.indent();
            self.output.push('}');
        }
        self.newline();
    }

    fn format_impl(&mut self, imp: &ImplBlock) {
        self.indent();
        self.output.push_str("impl ");
        self.format_generic_params(&imp.generic_params);
        if let Some(trait_name) = &imp.trait_name {
            self.output.push_str(trait_name);
            self.output.push_str(" for ");
        }
        self.output.push_str(&imp.type_name);
        self.output.push_str(" {");
        self.newline();
        self.indent_level += 1;

        for method in &imp.methods {
            self.format_function(method);
        }

        self.indent_level -= 1;
        self.indent();
        self.output.push('}');
        self.newline();
    }

    fn format_import(&mut self, imp: &ImportDecl) {
        self.indent();
        self.output.push_str("import ");
        self.output.push_str(&imp.path.join("::"));
        if let Some(items) = &imp.items {
            self.output.push_str("::{");
            self.output.push_str(&items.join(", "));
            self.output.push('}');
        }
        if let Some(alias) = &imp.alias {
            self.output.push_str(" as ");
            self.output.push_str(alias);
        }
        self.newline();
    }

    fn format_const(&mut self, c: &ConstDecl) {
        self.indent();
        if c.is_pub {
            self.output.push_str("pub ");
        }
        self.output.push_str("const ");
        self.output.push_str(&c.name);
        if let Some(ty) = &c.type_expr {
            self.output.push_str(": ");
            self.format_type_expr(ty);
        }
        self.output.push_str(" = ");
        self.format_expr(&c.value);
        self.newline();
    }

    fn format_type_alias(&mut self, ta: &TypeAlias) {
        self.indent();
        if ta.is_pub {
            self.output.push_str("pub ");
        }
        self.output.push_str("type ");
        self.output.push_str(&ta.name);
        self.format_generic_params(&ta.generic_params);
        self.output.push_str(" = ");
        self.format_type_expr(&ta.type_expr);
        self.newline();
    }

    fn format_module(&mut self, m: &Module) {
        self.indent();
        self.output.push_str("mod ");
        self.output.push_str(&m.name);
        self.output.push_str(" {");
        self.newline();
        self.indent_level += 1;

        for item in &m.items {
            self.format_item(item);
        }

        self.indent_level -= 1;
        self.indent();
        self.output.push('}');
        self.newline();
    }

    fn format_test(&mut self, t: &TestDecl) {
        self.indent();
        self.output.push_str("test \"");
        self.output.push_str(&t.name);
        self.output.push_str("\" {");
        self.newline();
        self.indent_level += 1;
        self.format_block(&t.body);
        self.indent_level -= 1;
        self.indent();
        self.output.push('}');
        self.newline();
    }

    fn format_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.format_stmt(stmt);
        }
    }

    fn format_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(l) => {
                self.indent();
                self.output.push_str("let ");
                if l.mutable {
                    self.output.push_str("mut ");
                }
                self.output.push_str(&l.name);
                if let Some(ty) = &l.type_expr {
                    self.output.push_str(": ");
                    self.format_type_expr(ty);
                }
                if let Some(val) = &l.value {
                    self.output.push_str(" = ");
                    self.format_expr(val);
                }
                self.newline();
            }
            Stmt::Expr(expr) => {
                self.indent();
                self.format_expr(expr);
                self.newline();
            }
            Stmt::Return(ret) => {
                self.indent();
                self.output.push_str("return");
                if let Some(val) = &ret.value {
                    self.output.push(' ');
                    self.format_expr(val);
                }
                self.newline();
            }
            Stmt::While(w) => {
                self.indent();
                self.output.push_str("while ");
                self.format_expr(&w.condition);
                self.output.push_str(" {");
                self.newline();
                self.indent_level += 1;
                self.format_block(&w.body);
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
                self.newline();
            }
            Stmt::For(f) => {
                self.indent();
                self.output.push_str("for ");
                self.output.push_str(&f.variable);
                self.output.push_str(" in ");
                self.format_expr(&f.iterator);
                self.output.push_str(" {");
                self.newline();
                self.indent_level += 1;
                self.format_block(&f.body);
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
                self.newline();
            }
            Stmt::Assign(a) => {
                self.indent();
                self.format_expr(&a.target);
                if let Some(op) = &a.op {
                    self.output.push(' ');
                    self.output.push_str(&format!("{}", op));
                    self.output.push_str("= ");
                } else {
                    self.output.push_str(" = ");
                }
                self.format_expr(&a.value);
                self.newline();
            }
            Stmt::Break(_) => {
                self.indent();
                self.output.push_str("break");
                self.newline();
            }
            Stmt::Continue(_) => {
                self.indent();
                self.output.push_str("continue");
                self.newline();
            }
            Stmt::Item(item) => {
                self.format_item(item);
            }
        }
    }

    fn format_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(lit) => self.format_literal(lit),
            Expr::Identifier(id) => self.output.push_str(&id.name),
            Expr::Binary(bin) => {
                self.format_expr(&bin.left);
                self.output.push(' ');
                self.output.push_str(&format!("{}", bin.op));
                self.output.push(' ');
                self.format_expr(&bin.right);
            }
            Expr::Unary(un) => {
                self.format_unary_op(&un.op);
                self.format_expr(&un.operand);
            }
            Expr::Call(call) => {
                self.format_expr(&call.callee);
                if !call.generic_args.is_empty() {
                    self.output.push_str("::<");
                    for (i, arg) in call.generic_args.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.format_type_expr(arg);
                    }
                    self.output.push('>');
                }
                self.output.push('(');
                for (i, arg) in call.args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_expr(arg);
                }
                self.output.push(')');
            }
            Expr::MethodCall(mc) => {
                self.format_expr(&mc.receiver);
                self.output.push('.');
                self.output.push_str(&mc.method);
                if !mc.generic_args.is_empty() {
                    self.output.push_str("::<");
                    for (i, arg) in mc.generic_args.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.format_type_expr(arg);
                    }
                    self.output.push('>');
                }
                self.output.push('(');
                for (i, arg) in mc.args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_expr(arg);
                }
                self.output.push(')');
            }
            Expr::FieldAccess(fa) => {
                self.format_expr(&fa.object);
                self.output.push('.');
                self.output.push_str(&fa.field);
            }
            Expr::Index(idx) => {
                self.format_expr(&idx.object);
                self.output.push('[');
                self.format_expr(&idx.index);
                self.output.push(']');
            }
            Expr::If(if_expr) => {
                self.output.push_str("if ");
                self.format_expr(&if_expr.condition);
                self.output.push_str(" {");
                self.newline();
                self.indent_level += 1;
                self.format_block(&if_expr.then_block);
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
                if let Some(else_expr) = &if_expr.else_block {
                    self.output.push_str(" else ");
                    match else_expr.as_ref() {
                        Expr::If(_) => self.format_expr(else_expr),
                        Expr::Block(block) => {
                            self.output.push('{');
                            self.newline();
                            self.indent_level += 1;
                            self.format_block(block);
                            self.indent_level -= 1;
                            self.indent();
                            self.output.push('}');
                        }
                        _ => {
                            self.output.push_str("{ ");
                            self.format_expr(else_expr);
                            self.output.push_str(" }");
                        }
                    }
                }
            }
            Expr::Match(m) => {
                self.output.push_str("match ");
                self.format_expr(&m.subject);
                self.output.push_str(" {");
                self.newline();
                self.indent_level += 1;
                for arm in &m.arms {
                    self.indent();
                    self.format_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.output.push_str(" if ");
                        self.format_expr(guard);
                    }
                    self.output.push_str(" => ");
                    self.format_expr(&arm.body);
                    self.output.push(',');
                    self.newline();
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            Expr::Block(block) => {
                self.output.push('{');
                self.newline();
                self.indent_level += 1;
                self.format_block(block);
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            Expr::Array(arr) => {
                self.output.push('[');
                for (i, elem) in arr.elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_expr(elem);
                }
                self.output.push(']');
            }
            Expr::Tuple(t) => {
                self.output.push('(');
                for (i, elem) in t.elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_expr(elem);
                }
                if t.elements.len() == 1 {
                    self.output.push(',');
                }
                self.output.push(')');
            }
            Expr::StructInit(si) => {
                self.output.push_str(&si.name);
                self.output.push_str(" {");
                for (i, (name, val)) in si.fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push(' ');
                    self.output.push_str(name);
                    self.output.push_str(": ");
                    self.format_expr(val);
                }
                self.output.push_str(" }");
            }
            Expr::Lambda(lam) => {
                self.output.push('|');
                for (i, param) in lam.params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_param(param);
                }
                self.output.push_str("| ");
                self.format_expr(&lam.body);
            }
            Expr::Await(a) => {
                self.format_expr(&a.expr);
                self.output.push_str(".await");
            }
            Expr::Range(r) => {
                if let Some(start) = &r.start {
                    self.format_expr(start);
                }
                if r.inclusive {
                    self.output.push_str("..=");
                } else {
                    self.output.push_str("..");
                }
                if let Some(end) = &r.end {
                    self.format_expr(end);
                }
            }
            Expr::Cast(c) => {
                self.format_expr(&c.expr);
                self.output.push_str(" as ");
                self.format_type_expr(&c.type_expr);
            }
        }
    }

    fn format_literal(&mut self, lit: &Literal) {
        match &lit.kind {
            LiteralKind::Int(n) => self.output.push_str(&n.to_string()),
            LiteralKind::Float(f) => {
                let s = f.to_string();
                self.output.push_str(&s);
                if !s.contains('.') {
                    self.output.push_str(".0");
                }
            }
            LiteralKind::String(s) => {
                self.output.push('"');
                for ch in s.chars() {
                    match ch {
                        '"' => self.output.push_str("\\\""),
                        '\\' => self.output.push_str("\\\\"),
                        '\n' => self.output.push_str("\\n"),
                        '\r' => self.output.push_str("\\r"),
                        '\t' => self.output.push_str("\\t"),
                        c => self.output.push(c),
                    }
                }
                self.output.push('"');
            }
            LiteralKind::Bool(b) => {
                self.output.push_str(if *b { "true" } else { "false" });
            }
            LiteralKind::Char(c) => {
                self.output.push('\'');
                match c {
                    '\'' => self.output.push_str("\\'"),
                    '\\' => self.output.push_str("\\\\"),
                    '\n' => self.output.push_str("\\n"),
                    '\r' => self.output.push_str("\\r"),
                    '\t' => self.output.push_str("\\t"),
                    c => self.output.push(*c),
                }
                self.output.push('\'');
            }
            LiteralKind::None => {
                self.output.push_str("none");
            }
        }
    }

    fn format_unary_op(&mut self, op: &UnaryOp) {
        match op {
            UnaryOp::Neg => self.output.push('-'),
            UnaryOp::Not => self.output.push('!'),
            UnaryOp::BitNot => self.output.push('~'),
            UnaryOp::Ref => self.output.push('&'),
            UnaryOp::RefMut => self.output.push_str("&mut "),
            UnaryOp::Deref => self.output.push('*'),
        }
    }

    fn format_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Identifier(name, _) => self.output.push_str(name),
            Pattern::Literal(lit) => self.format_literal(lit),
            Pattern::Tuple(pats, _) => {
                self.output.push('(');
                for (i, p) in pats.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_pattern(p);
                }
                self.output.push(')');
            }
            Pattern::Struct { name, fields, .. } => {
                self.output.push_str(name);
                self.output.push_str(" { ");
                for (i, (fname, fpat)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(fname);
                    self.output.push_str(": ");
                    self.format_pattern(fpat);
                }
                self.output.push_str(" }");
            }
            Pattern::Enum {
                name,
                variant,
                fields,
                ..
            } => {
                self.output.push_str(name);
                self.output.push_str("::");
                self.output.push_str(variant);
                if !fields.is_empty() {
                    self.output.push('(');
                    for (i, p) in fields.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.format_pattern(p);
                    }
                    self.output.push(')');
                }
            }
            Pattern::Wildcard(_) => self.output.push('_'),
            Pattern::Or(pats, _) => {
                for (i, p) in pats.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(" | ");
                    }
                    self.format_pattern(p);
                }
            }
        }
    }

    fn format_type_expr(&mut self, ty: &TypeExpr) {
        match ty {
            TypeExpr::Named {
                name, generic_args, ..
            } => {
                self.output.push_str(name);
                if !generic_args.is_empty() {
                    self.output.push('<');
                    for (i, arg) in generic_args.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.format_type_expr(arg);
                    }
                    self.output.push('>');
                }
            }
            TypeExpr::Array { element, size, .. } => {
                self.output.push('[');
                self.format_type_expr(element);
                if let Some(sz) = size {
                    self.output.push_str("; ");
                    self.format_expr(sz);
                }
                self.output.push(']');
            }
            TypeExpr::Tuple { elements, .. } => {
                self.output.push('(');
                for (i, item) in elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_type_expr(item);
                }
                self.output.push(')');
            }
            TypeExpr::Function {
                params,
                return_type,
                ..
            } => {
                self.output.push_str("fn(");
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_type_expr(param);
                }
                self.output.push_str(") -> ");
                self.format_type_expr(return_type);
            }
            TypeExpr::Optional { inner, .. } => {
                self.format_type_expr(inner);
                self.output.push('?');
            }
            TypeExpr::Reference { inner, mutable, .. } => {
                self.output.push('&');
                if *mutable {
                    self.output.push_str("mut ");
                }
                self.format_type_expr(inner);
            }
        }
    }
}
