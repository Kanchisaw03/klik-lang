// KLIK Linter - Static analysis rules

use klik_ast::*;
use std::collections::HashMap;

/// Diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A lint diagnostic
#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    pub severity: Severity,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub rule: &'static str,
}

/// The linter
pub struct Linter {
    diagnostics: Vec<LintDiagnostic>,
}

impl Linter {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// Run all lint rules on a program and return diagnostics
    pub fn lint(&mut self, program: &Program) -> Vec<LintDiagnostic> {
        self.diagnostics.clear();

        for module in &program.modules {
            for item in &module.items {
                self.lint_item(item);
            }
        }

        self.diagnostics.clone()
    }

    fn lint_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => self.lint_function(f),
            Item::Struct(s) => self.lint_struct(s),
            Item::Enum(e) => self.lint_enum(e),
            Item::Trait(t) => {
                self.check_type_naming(&t.name, &t.span, "Trait");
                for method in &t.methods {
                    self.check_snake_case(&method.name, &method.span, "Trait method");
                    for param in &method.params {
                        self.check_snake_case(&param.name, &param.span, "Parameter");
                    }
                    if let Some(body) = &method.default_body {
                        self.lint_block(body);
                    }
                }
            }
            Item::Impl(i) => {
                for method in &i.methods {
                    self.lint_function(method);
                }
            }
            Item::Module(m) => {
                self.check_snake_case(&m.name, &m.span, "Module");
                for item in &m.items {
                    self.lint_item(item);
                }
            }
            Item::Const(c) => {
                self.check_screaming_snake_case(&c.name, &c.span, "Constant");
                self.lint_expr(&c.value);
            }
            Item::Test(t) => {
                self.lint_block(&t.body);
            }
            Item::Import(_) | Item::TypeAlias(_) => {}
        }
    }

    fn lint_function(&mut self, func: &Function) {
        if func.name != "main" {
            self.check_snake_case(&func.name, &func.span, "Function");
        }

        if func.body.stmts.is_empty() {
            self.warn(
                &func.span,
                &format!("Function `{}` has an empty body", func.name),
                "empty-function",
            );
        }

        if func.params.len() > 7 {
            self.warn(
                &func.span,
                &format!(
                    "Function `{}` has {} parameters (consider using a struct)",
                    func.name,
                    func.params.len()
                ),
                "too-many-params",
            );
        }

        if func.body.stmts.len() > 60 {
            self.warn(
                &func.span,
                &format!(
                    "Function `{}` has {} statements (consider splitting it)",
                    func.name,
                    func.body.stmts.len()
                ),
                "long-function",
            );
        }

        for param in &func.params {
            self.check_snake_case(&param.name, &param.span, "Parameter");
        }

        self.lint_block(&func.body);
        self.lint_variable_usage(func);
    }

    fn lint_variable_usage(&mut self, func: &Function) {
        let mut scopes: Vec<HashMap<String, (bool, Span)>> = vec![HashMap::new()];

        for param in &func.params {
            if let Some(root) = scopes.last_mut() {
                root.insert(param.name.clone(), (true, param.span.clone()));
            }
        }

        self.track_block_vars(&func.body, &mut scopes, false);
        if let Some(root) = scopes.pop() {
            self.report_unused_scope(root);
        }
    }

    fn track_block_vars(
        &mut self,
        block: &Block,
        scopes: &mut Vec<HashMap<String, (bool, Span)>>,
        create_scope: bool,
    ) {
        if create_scope {
            scopes.push(HashMap::new());
        }

        for stmt in &block.stmts {
            self.track_stmt_vars(stmt, scopes);
        }

        if create_scope {
            if let Some(scope) = scopes.pop() {
                self.report_unused_scope(scope);
            }
        }
    }

    fn track_stmt_vars(&mut self, stmt: &Stmt, scopes: &mut Vec<HashMap<String, (bool, Span)>>) {
        match stmt {
            Stmt::Let(l) => {
                if let Some(val) = &l.value {
                    self.track_expr_vars(val, scopes);
                }

                if scopes.iter().rev().any(|scope| scope.contains_key(&l.name)) {
                    self.warn(
                        &l.span,
                        &format!("Variable `{}` shadows an existing binding", l.name),
                        "shadowing",
                    );
                }

                if let Some(scope) = scopes.last_mut() {
                    scope.insert(l.name.clone(), (false, l.span.clone()));
                }
            }
            Stmt::Expr(expr) => self.track_expr_vars(expr, scopes),
            Stmt::Return(r) => {
                if let Some(val) = &r.value {
                    self.track_expr_vars(val, scopes);
                }
            }
            Stmt::While(w) => {
                self.track_expr_vars(&w.condition, scopes);
                self.track_block_vars(&w.body, scopes, true);
            }
            Stmt::For(f) => {
                self.track_expr_vars(&f.iterator, scopes);
                scopes.push(HashMap::new());
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(f.variable.clone(), (false, f.span.clone()));
                }
                self.track_block_vars(&f.body, scopes, false);
                if let Some(scope) = scopes.pop() {
                    self.report_unused_scope(scope);
                }
            }
            Stmt::Assign(a) => {
                self.track_expr_vars(&a.target, scopes);
                self.track_expr_vars(&a.value, scopes);
            }
            Stmt::Item(_) | Stmt::Break(_) | Stmt::Continue(_) => {}
        }
    }

    fn track_expr_vars(&mut self, expr: &Expr, scopes: &mut Vec<HashMap<String, (bool, Span)>>) {
        match expr {
            Expr::Identifier(ident) => {
                for scope in scopes.iter_mut().rev() {
                    if let Some((used, _)) = scope.get_mut(&ident.name) {
                        *used = true;
                        break;
                    }
                }
            }
            Expr::Binary(bin) => {
                self.track_expr_vars(&bin.left, scopes);
                self.track_expr_vars(&bin.right, scopes);
            }
            Expr::Unary(un) => self.track_expr_vars(&un.operand, scopes),
            Expr::Call(call) => {
                self.track_expr_vars(&call.callee, scopes);
                for arg in &call.args {
                    self.track_expr_vars(arg, scopes);
                }
            }
            Expr::MethodCall(mc) => {
                self.track_expr_vars(&mc.receiver, scopes);
                for arg in &mc.args {
                    self.track_expr_vars(arg, scopes);
                }
            }
            Expr::FieldAccess(fa) => self.track_expr_vars(&fa.object, scopes),
            Expr::Index(idx) => {
                self.track_expr_vars(&idx.object, scopes);
                self.track_expr_vars(&idx.index, scopes);
            }
            Expr::If(if_expr) => {
                self.track_expr_vars(&if_expr.condition, scopes);
                self.track_block_vars(&if_expr.then_block, scopes, true);
                if let Some(else_expr) = &if_expr.else_block {
                    self.track_expr_vars(else_expr, scopes);
                }
            }
            Expr::Match(m) => {
                self.track_expr_vars(&m.subject, scopes);
                for arm in &m.arms {
                    if let Some(guard) = &arm.guard {
                        self.track_expr_vars(guard, scopes);
                    }
                    self.track_expr_vars(&arm.body, scopes);
                }
            }
            Expr::Block(block) => self.track_block_vars(block, scopes, true),
            Expr::Array(arr) => {
                for elem in &arr.elements {
                    self.track_expr_vars(elem, scopes);
                }
            }
            Expr::Tuple(tuple) => {
                for elem in &tuple.elements {
                    self.track_expr_vars(elem, scopes);
                }
            }
            Expr::StructInit(si) => {
                for (_, val) in &si.fields {
                    self.track_expr_vars(val, scopes);
                }
            }
            Expr::Lambda(lam) => {
                scopes.push(HashMap::new());
                if let Some(scope) = scopes.last_mut() {
                    for param in &lam.params {
                        scope.insert(param.name.clone(), (false, param.span.clone()));
                    }
                }
                self.track_expr_vars(&lam.body, scopes);
                if let Some(scope) = scopes.pop() {
                    self.report_unused_scope(scope);
                }
            }
            Expr::Await(a) => self.track_expr_vars(&a.expr, scopes),
            Expr::Range(r) => {
                if let Some(start) = &r.start {
                    self.track_expr_vars(start, scopes);
                }
                if let Some(end) = &r.end {
                    self.track_expr_vars(end, scopes);
                }
            }
            Expr::Cast(c) => self.track_expr_vars(&c.expr, scopes),
            Expr::Literal(_) => {}
        }
    }

    fn report_unused_scope(&mut self, scope: HashMap<String, (bool, Span)>) {
        for (name, (used, span)) in scope {
            if !used && !name.starts_with('_') {
                self.warn(
                    &span,
                    &format!("Unused variable `{}`", name),
                    "unused-variable",
                );
            }
        }
    }

    fn lint_struct(&mut self, s: &StructDef) {
        self.check_type_naming(&s.name, &s.span, "Struct");

        for field in &s.fields {
            self.check_snake_case(&field.name, &field.span, "Field");
        }

        if s.fields.is_empty() {
            self.info(
                &s.span,
                &format!("Struct `{}` has no fields", s.name),
                "empty-struct",
            );
        }
    }

    fn lint_enum(&mut self, e: &EnumDef) {
        self.check_type_naming(&e.name, &e.span, "Enum");

        for variant in &e.variants {
            self.check_type_naming(&variant.name, &variant.span, "Enum variant");
        }

        if e.variants.len() == 1 {
            self.warn(
                &e.span,
                &format!(
                    "Enum `{}` has only one variant (consider using a struct)",
                    e.name
                ),
                "single-variant-enum",
            );
        }
    }

    fn lint_block(&mut self, block: &Block) {
        // Check for consecutive duplicate expression statements
        for window in block.stmts.windows(2) {
            if let (Stmt::Expr(a), Stmt::Expr(b)) = (&window[0], &window[1]) {
                if format!("{:?}", a) == format!("{:?}", b) {
                    self.warn(
                        a.span(),
                        "Consecutive identical expressions (possible copy-paste error)",
                        "duplicate-expression",
                    );
                }
            }
        }

        for stmt in &block.stmts {
            self.lint_stmt(stmt);
        }
    }

    fn lint_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(l) => {
                self.check_snake_case(&l.name, &l.span, "Variable");
                if let Some(val) = &l.value {
                    self.lint_expr(val);
                }
            }
            Stmt::Expr(expr) => {
                self.lint_expr(expr);
            }
            Stmt::While(w) => {
                self.lint_expr(&w.condition);
                if let Expr::Literal(Literal {
                    kind: LiteralKind::Bool(true),
                    ..
                }) = &w.condition
                {
                    self.info(
                        &w.span,
                        "Infinite loop: condition is always `true`",
                        "infinite-loop",
                    );
                }
                self.lint_block(&w.body);
            }
            Stmt::For(f) => {
                self.check_snake_case(&f.variable, &f.span, "Loop variable");
                self.lint_expr(&f.iterator);
                self.lint_block(&f.body);
            }
            Stmt::Return(r) => {
                if let Some(val) = &r.value {
                    self.lint_expr(val);
                }
            }
            Stmt::Assign(a) => {
                self.lint_expr(&a.target);
                self.lint_expr(&a.value);
            }
            Stmt::Item(item) => {
                self.lint_item(item);
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
        }
    }

    fn lint_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Binary(bin) => {
                // Check for comparison with self
                if matches!(
                    bin.op,
                    BinaryOp::Eq | BinaryOp::Neq | BinaryOp::Lt | BinaryOp::Gt
                ) && format!("{:?}", bin.left) == format!("{:?}", bin.right)
                {
                    self.warn(
                        &bin.span,
                        "Comparing expression with itself",
                        "self-comparison",
                    );
                }

                // Check for division by zero
                if bin.op == BinaryOp::Div || bin.op == BinaryOp::Mod {
                    if let Expr::Literal(Literal {
                        kind: LiteralKind::Int(0),
                        ..
                    }) = bin.right.as_ref()
                    {
                        self.error(&bin.span, "Division by zero", "division-by-zero");
                    }
                }

                // Check for unnecessary boolean comparisons
                if bin.op == BinaryOp::Eq {
                    if let Expr::Literal(Literal {
                        kind: LiteralKind::Bool(true),
                        ..
                    }) = bin.right.as_ref()
                    {
                        self.warn(
                            &bin.span,
                            "Unnecessary comparison with `true`",
                            "bool-comparison",
                        );
                    }
                    if let Expr::Literal(Literal {
                        kind: LiteralKind::Bool(false),
                        ..
                    }) = bin.right.as_ref()
                    {
                        self.warn(
                            &bin.span,
                            "Use `!expr` instead of `expr == false`",
                            "bool-comparison",
                        );
                    }
                }

                self.lint_expr(&bin.left);
                self.lint_expr(&bin.right);
            }
            Expr::Unary(un) => {
                self.lint_expr(&un.operand);
            }
            Expr::Call(call) => {
                self.lint_expr(&call.callee);
                for arg in &call.args {
                    self.lint_expr(arg);
                }
            }
            Expr::MethodCall(mc) => {
                self.lint_expr(&mc.receiver);
                for arg in &mc.args {
                    self.lint_expr(arg);
                }
            }
            Expr::If(if_expr) => {
                self.lint_expr(&if_expr.condition);
                self.lint_block(&if_expr.then_block);
                if let Some(else_expr) = &if_expr.else_block {
                    self.lint_expr(else_expr);
                }
            }
            Expr::Match(m) => {
                self.lint_expr(&m.subject);
                if m.arms.is_empty() {
                    self.warn(&m.span, "Empty match expression", "empty-match");
                }
                for arm in &m.arms {
                    self.lint_expr(&arm.body);
                    if let Some(guard) = &arm.guard {
                        self.lint_expr(guard);
                    }
                }
            }
            Expr::Block(block) => {
                self.lint_block(block);
            }
            Expr::Lambda(lam) => {
                for param in &lam.params {
                    self.check_snake_case(&param.name, &param.span, "Lambda parameter");
                }
                self.lint_expr(&lam.body);
            }
            Expr::FieldAccess(fa) => {
                self.lint_expr(&fa.object);
            }
            Expr::Index(idx) => {
                self.lint_expr(&idx.object);
                self.lint_expr(&idx.index);
            }
            Expr::Array(arr) => {
                for elem in &arr.elements {
                    self.lint_expr(elem);
                }
            }
            Expr::Tuple(t) => {
                for elem in &t.elements {
                    self.lint_expr(elem);
                }
            }
            Expr::StructInit(si) => {
                for (_, val) in &si.fields {
                    self.lint_expr(val);
                }
            }
            Expr::Await(a) => {
                self.lint_expr(&a.expr);
            }
            Expr::Range(r) => {
                if let Some(start) = &r.start {
                    self.lint_expr(start);
                }
                if let Some(end) = &r.end {
                    self.lint_expr(end);
                }
            }
            Expr::Cast(c) => {
                self.lint_expr(&c.expr);
            }
            Expr::Literal(_) | Expr::Identifier(_) => {}
        }
    }

    // --- Naming convention checks ---

    fn check_snake_case(&mut self, name: &str, span: &Span, kind: &str) {
        if name.starts_with('_') {
            return;
        }
        if name.contains(char::is_uppercase) && !name.contains('_') && name.len() > 1 {
            self.warn(
                span,
                &format!("{} `{}` should be snake_case", kind, name),
                "naming-convention",
            );
        }
    }

    fn check_type_naming(&mut self, name: &str, span: &Span, kind: &str) {
        if let Some(first) = name.chars().next() {
            if !first.is_uppercase() {
                self.warn(
                    span,
                    &format!("{} `{}` should start with an uppercase letter", kind, name),
                    "naming-convention",
                );
            }
        }
    }

    fn check_screaming_snake_case(&mut self, name: &str, span: &Span, kind: &str) {
        if name.chars().any(|c| c.is_lowercase()) {
            self.warn(
                span,
                &format!("{} `{}` should be SCREAMING_SNAKE_CASE", kind, name),
                "naming-convention",
            );
        }
    }

    // --- Diagnostic helpers ---

    fn error(&mut self, span: &Span, message: &str, rule: &'static str) {
        self.diagnostics.push(LintDiagnostic {
            severity: Severity::Error,
            message: message.to_string(),
            line: span.start.line,
            column: span.start.column,
            rule,
        });
    }

    fn warn(&mut self, span: &Span, message: &str, rule: &'static str) {
        self.diagnostics.push(LintDiagnostic {
            severity: Severity::Warning,
            message: message.to_string(),
            line: span.start.line,
            column: span.start.column,
            rule,
        });
    }

    fn info(&mut self, span: &Span, message: &str, rule: &'static str) {
        self.diagnostics.push(LintDiagnostic {
            severity: Severity::Info,
            message: message.to_string(),
            line: span.start.line,
            column: span.start.column,
            rule,
        });
    }
}

impl Default for Linter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_span() -> Span {
        Span::dummy()
    }

    #[test]
    fn test_snake_case_detection() {
        let mut linter = Linter::new();
        linter.check_snake_case("camelCase", &make_span(), "Variable");
        assert_eq!(linter.diagnostics.len(), 1);
        assert_eq!(linter.diagnostics[0].rule, "naming-convention");
    }

    #[test]
    fn test_valid_snake_case() {
        let mut linter = Linter::new();
        linter.check_snake_case("my_variable", &make_span(), "Variable");
        assert_eq!(linter.diagnostics.len(), 0);
    }

    #[test]
    fn test_type_naming() {
        let mut linter = Linter::new();
        linter.check_type_naming("myStruct", &make_span(), "Struct");
        assert_eq!(linter.diagnostics.len(), 1);
    }

    fn lint_source(source: &str) -> Vec<LintDiagnostic> {
        let tokens = klik_lexer::Lexer::new(source, "<test>")
            .tokenize()
            .expect("tokenize");
        let mut parser = klik_parser::Parser::new(tokens, "<test>");
        let program = parser.parse_program().expect("parse");
        let mut linter = Linter::new();
        linter.lint(&program)
    }

    #[test]
    fn test_unused_variable_rule() {
        let diagnostics = lint_source(
            r#"
fn main() {
    let x = 10
}
"#,
        );
        assert!(diagnostics
            .iter()
            .any(|d| d.rule == "unused-variable" && d.message.contains("x")));
    }

    #[test]
    fn test_shadowing_rule() {
        let diagnostics = lint_source(
            r#"
fn main() {
    let x = 1
    let x = 2
    x
}
"#,
        );
        assert!(diagnostics
            .iter()
            .any(|d| d.rule == "shadowing" && d.message.contains("x")));
    }

    #[test]
    fn test_long_function_rule() {
        let mut body = String::new();
        for i in 0..61 {
            body.push_str(&format!("    let v{} = {}\n", i, i));
        }
        let source = format!("fn big() {{\n{}}}\n", body);
        let diagnostics = lint_source(&source);
        assert!(diagnostics.iter().any(|d| d.rule == "long-function"));
    }
}
