// KLIK CLI - Command implementations

use anyhow::{bail, Context, Result};
use colored::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

fn trace_log(enabled: bool, message: &str) {
    if enabled {
        eprintln!("{}", message);
    }
}

struct FrontendArtifacts {
    ast: klik_ast::Program,
    ir_module: Option<klik_ir::IrModule>,
}

fn parse_target(target: &str) -> Result<klik_codegen::Target> {
    match target {
        "native" => Ok(klik_codegen::Target::Native),
        "wasm" | "wasm32" => Ok(klik_codegen::Target::Wasm),
        "x86_64-linux" => Ok(klik_codegen::Target::X86_64Linux),
        "x86_64-windows" => Ok(klik_codegen::Target::X86_64Windows),
        "x86_64-macos" => Ok(klik_codegen::Target::X86_64MacOS),
        "aarch64-linux" => Ok(klik_codegen::Target::Aarch64Linux),
        "aarch64-macos" => Ok(klik_codegen::Target::Aarch64MacOS),
        other => bail!("Unknown target: {}", other),
    }
}

fn compile_frontend(
    source_code: &str,
    file_name: &str,
    module_name: &str,
    build_ir: bool,
    opt_level: crate::pipeline::CliOptLevel,
    trace: bool,
) -> Result<FrontendArtifacts> {
    trace_log(trace, "[PARSE] tokenizing source");
    let tokens = klik_lexer::Lexer::new(source_code, file_name)
        .tokenize()
        .map_err(|e| format_compiler_error("lexer", &format!("{:?}", e), source_code, file_name))?;

    trace_log(trace, "[PARSE] parsing AST");
    let ast = klik_parser::Parser::new(tokens, file_name)
        .parse_program()
        .map_err(|e| {
            format_compiler_error("parser", &format!("{:?}", e), source_code, file_name)
        })?;
    trace_log(trace, "[PARSE] AST built successfully");

    let mut analyzer = klik_semantic::SemanticAnalyzer::new();
    if let Err(errors) = analyzer.analyze(&ast) {
        let mut msgs = Vec::new();
        for e in &errors {
            msgs.push(format_semantic_error(e, source_code, file_name));
        }
        bail!("{}", msgs.join("\n"));
    }

    let mut type_checker = klik_type_system::TypeChecker::new();
    if let Err(errors) = type_checker.check_program(&ast) {
        let mut msgs = Vec::new();
        for e in &errors {
            msgs.push(format_type_error(e, source_code, file_name));
        }
        bail!("{}", msgs.join("\n"));
    }

    let ir_module = if build_ir {
        trace_log(trace, "[IR] building IR module");
        let mut ir_builder = klik_ir::IrBuilder::new(module_name);
        let mut ir_module = ir_builder.build_module(&ast);
        trace_log(trace, "[IR] IR module generated");

        let _report = crate::pipeline::run_optimization_pipeline(&mut ir_module, opt_level, trace);

        // The current IR builder may emit return values from expression tails even
        // when a function is declared as void; normalize these for backend validity.
        for func in &mut ir_module.functions {
            if func.return_type == klik_ir::IrType::Void {
                for block in &mut func.blocks {
                    if let Some(klik_ir::Terminator::Return(ret_val)) = &mut block.terminator {
                        *ret_val = None;
                    }
                }
            }
        }

        Some(ir_module)
    } else {
        None
    };

    Ok(FrontendArtifacts { ast, ir_module })
}

/// Format a compiler error with source context
fn format_compiler_error(stage: &str, message: &str, source: &str, file: &str) -> anyhow::Error {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = format!(
        "{}: {} error\n  {} {}\n",
        "error".red().bold(),
        stage,
        "-->".blue().bold(),
        file
    );
    // Try to extract line number from the message
    if let Some(line_num) = extract_line_from_message(message) {
        if line_num > 0 && line_num <= lines.len() {
            let pad = format!("{}", line_num).len();
            out.push_str(&format!("{}|\n", " ".repeat(pad + 1)));
            out.push_str(&format!(
                "{} | {}\n",
                format!("{:>width$}", line_num, width = pad).blue().bold(),
                lines[line_num - 1]
            ));
            out.push_str(&format!("{}|\n", " ".repeat(pad + 1)));
        }
    }
    out.push_str(&format!("  = {}", message));
    anyhow::anyhow!("{}", out)
}

fn format_semantic_error(
    error: &klik_semantic::SemanticError,
    source: &str,
    _file: &str,
) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let span = get_semantic_error_span(error);
    let line_num = span.map(|s| s.start.line + 1).unwrap_or(0);
    let col = span.map(|s| s.start.column + 1).unwrap_or(0);

    let mut out = format!(
        "{}: {}\n  {} {}:{}:{}\n",
        "error".red().bold(),
        error,
        "-->".blue().bold(),
        span.map(|s| s.file.as_str()).unwrap_or("unknown"),
        line_num,
        col
    );

    if line_num > 0 && line_num <= lines.len() {
        let pad = format!("{}", line_num).len();
        out.push_str(&format!("{}|\n", " ".repeat(pad + 1)));
        out.push_str(&format!(
            "{} | {}\n",
            format!("{:>width$}", line_num, width = pad).blue().bold(),
            lines[line_num - 1]
        ));
        // Add pointer
        if col > 0 {
            out.push_str(&format!(
                "{}| {}{}\n",
                " ".repeat(pad + 1),
                " ".repeat(col - 1),
                "^".red().bold()
            ));
        }
    }

    // Add suggestion hint
    if let Some(hint) = get_semantic_hint(error) {
        out.push_str(&format!("  = {}: {}\n", "hint".cyan().bold(), hint));
    }

    out
}

fn format_type_error(error: &klik_type_system::TypeError, source: &str, _file: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let span = get_type_error_span(error);
    let line_num = span.map(|s| s.start.line + 1).unwrap_or(0);
    let col = span.map(|s| s.start.column + 1).unwrap_or(0);

    let mut out = format!(
        "{}: {}\n  {} {}:{}:{}\n",
        "error".red().bold(),
        error,
        "-->".blue().bold(),
        span.map(|s| s.file.as_str()).unwrap_or("unknown"),
        line_num,
        col
    );

    if line_num > 0 && line_num <= lines.len() {
        let pad = format!("{}", line_num).len();
        out.push_str(&format!("{}|\n", " ".repeat(pad + 1)));
        out.push_str(&format!(
            "{} | {}\n",
            format!("{:>width$}", line_num, width = pad).blue().bold(),
            lines[line_num - 1]
        ));
        if col > 0 {
            out.push_str(&format!(
                "{}| {}{}\n",
                " ".repeat(pad + 1),
                " ".repeat(col - 1),
                "^".red().bold()
            ));
        }
    }

    out
}

fn extract_line_from_message(msg: &str) -> Option<usize> {
    // Try to find "line X" or ":X:" pattern
    for part in msg.split(|c: char| !c.is_ascii_digit()) {
        if let Ok(n) = part.parse::<usize>() {
            if n > 0 && n < 100000 {
                return Some(n);
            }
        }
    }
    None
}

fn get_semantic_error_span(error: &klik_semantic::SemanticError) -> Option<&klik_ast::Span> {
    match error {
        klik_semantic::SemanticError::UndefinedSymbol { span, .. } => Some(span),
        klik_semantic::SemanticError::DuplicateDefinition { span, .. } => Some(span),
        klik_semantic::SemanticError::ImmutableMutation { span, .. } => Some(span),
        klik_semantic::SemanticError::BreakOutsideLoop { span } => Some(span),
        klik_semantic::SemanticError::ReturnOutsideFunction { span } => Some(span),
        klik_semantic::SemanticError::General { span, .. } => Some(span),
        klik_semantic::SemanticError::TypeError(_) => None,
    }
}

fn get_semantic_hint(error: &klik_semantic::SemanticError) -> Option<String> {
    match error {
        klik_semantic::SemanticError::UndefinedSymbol { name, .. } => {
            Some(format!("did you mean to define '{}' or import it?", name))
        }
        klik_semantic::SemanticError::ImmutableMutation { name, .. } => Some(format!(
            "consider declaring '{name}' as mutable: `let mut {name} = ...`"
        )),
        klik_semantic::SemanticError::BreakOutsideLoop { .. } => {
            Some("break/continue can only be used inside while/for loops".into())
        }
        klik_semantic::SemanticError::ReturnOutsideFunction { .. } => {
            Some("return statements must be inside a function body".into())
        }
        _ => None,
    }
}

fn get_type_error_span(error: &klik_type_system::TypeError) -> Option<&klik_ast::Span> {
    match error {
        klik_type_system::TypeError::Mismatch { span, .. } => Some(span),
        klik_type_system::TypeError::UndefinedType { span, .. } => Some(span),
        klik_type_system::TypeError::CannotInfer { span } => Some(span),
        klik_type_system::TypeError::General { span, .. } => Some(span),
    }
}

fn default_binary_path(cwd: &Path, name: &str) -> PathBuf {
    if cfg!(windows) {
        cwd.join(format!("{}.exe", name))
    } else {
        cwd.join(name)
    }
}

fn default_project_output_path(cwd: &Path, release: bool, project_name: &str) -> PathBuf {
    let target_dir = cwd
        .join("target")
        .join(if release { "release" } else { "debug" });
    if cfg!(windows) {
        target_dir.join(format!("{}.exe", project_name))
    } else {
        target_dir.join(project_name)
    }
}

fn build_artifact_dir(cwd: &Path) -> PathBuf {
    cwd.join("target").join("build")
}

fn default_object_path(cwd: &Path, name: &str) -> PathBuf {
    build_artifact_dir(cwd).join(format!("{}.obj", name))
}

fn default_ir_artifact_path(cwd: &Path, name: &str) -> PathBuf {
    build_artifact_dir(cwd).join(format!("{}.ir", name))
}

fn ensure_executable_path(target: &str, output_path: PathBuf) -> PathBuf {
    if target == "x86_64-windows"
        || target == "native"
        || target == "x86_64-linux"
        || target == "x86_64-macos"
        || target == "aarch64-linux"
        || target == "aarch64-macos"
    {
        if output_path.extension().is_none() && cfg!(windows) {
            return output_path.with_extension("exe");
        }
    }
    output_path
}

fn dot_escape(label: &str) -> String {
    label
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\l")
}

fn ast_to_dot(program: &klik_ast::Program) -> String {
    struct AstDotBuilder {
        next_id: usize,
        nodes: Vec<String>,
        edges: Vec<String>,
    }

    impl AstDotBuilder {
        fn new_node(&mut self, label: &str) -> usize {
            let id = self.next_id;
            self.next_id += 1;
            self.nodes
                .push(format!("  n{} [label=\"{}\"]", id, dot_escape(label)));
            id
        }

        fn edge(&mut self, from: usize, to: usize) {
            self.edges.push(format!("  n{} -> n{}", from, to));
        }

        fn walk_program(&mut self, p: &klik_ast::Program) -> usize {
            let root = self.new_node("Program");
            for m in &p.modules {
                let child = self.walk_module(m);
                self.edge(root, child);
            }
            root
        }

        fn walk_module(&mut self, m: &klik_ast::Module) -> usize {
            let node = self.new_node(&format!("mod {}", m.name));
            for item in &m.items {
                let child = self.walk_item(item);
                self.edge(node, child);
            }
            node
        }

        fn walk_item(&mut self, item: &klik_ast::Item) -> usize {
            match item {
                klik_ast::Item::Function(f) => {
                    let node = self.new_node(&format!("fn {}", f.name));
                    let body = self.walk_block(&f.body);
                    self.edge(node, body);
                    node
                }
                klik_ast::Item::Struct(s) => {
                    let node = self.new_node(&format!("struct {}", s.name));
                    for field in &s.fields {
                        let f = self.new_node(&format!("field {}", field.name));
                        self.edge(node, f);
                    }
                    node
                }
                klik_ast::Item::Enum(e) => {
                    let node = self.new_node(&format!("enum {}", e.name));
                    for v in &e.variants {
                        let c = self.new_node(&format!("variant {}", v.name));
                        self.edge(node, c);
                    }
                    node
                }
                _ => self.new_node("item"),
            }
        }

        fn walk_block(&mut self, b: &klik_ast::Block) -> usize {
            let node = self.new_node("block");
            for stmt in &b.stmts {
                let c = self.walk_stmt(stmt);
                self.edge(node, c);
            }
            node
        }

        fn walk_stmt(&mut self, s: &klik_ast::Stmt) -> usize {
            match s {
                klik_ast::Stmt::Let(l) => {
                    let node = self.new_node(&format!("let {}", l.name));
                    if let Some(v) = &l.value {
                        let c = self.walk_expr(v);
                        self.edge(node, c);
                    }
                    node
                }
                klik_ast::Stmt::Expr(e) => self.walk_expr(e),
                klik_ast::Stmt::Return(r) => {
                    let node = self.new_node("return");
                    if let Some(v) = &r.value {
                        let c = self.walk_expr(v);
                        self.edge(node, c);
                    }
                    node
                }
                klik_ast::Stmt::While(w) => {
                    let node = self.new_node("while");
                    let cond = self.walk_expr(&w.condition);
                    let body = self.walk_block(&w.body);
                    self.edge(node, cond);
                    self.edge(node, body);
                    node
                }
                klik_ast::Stmt::For(f) => {
                    let node = self.new_node(&format!("for {}", f.variable));
                    let iter = self.walk_expr(&f.iterator);
                    let body = self.walk_block(&f.body);
                    self.edge(node, iter);
                    self.edge(node, body);
                    node
                }
                klik_ast::Stmt::Assign(a) => {
                    let node = self.new_node("assign");
                    let lhs = self.walk_expr(&a.target);
                    let rhs = self.walk_expr(&a.value);
                    self.edge(node, lhs);
                    self.edge(node, rhs);
                    node
                }
                klik_ast::Stmt::Break(_) => self.new_node("break"),
                klik_ast::Stmt::Continue(_) => self.new_node("continue"),
                klik_ast::Stmt::Item(i) => self.walk_item(i),
            }
        }

        fn walk_expr(&mut self, e: &klik_ast::Expr) -> usize {
            match e {
                klik_ast::Expr::Literal(_) => self.new_node("literal"),
                klik_ast::Expr::Identifier(i) => self.new_node(&format!("id {}", i.name)),
                klik_ast::Expr::Binary(b) => {
                    let node = self.new_node(&format!("bin {:?}", b.op));
                    let l = self.walk_expr(&b.left);
                    let r = self.walk_expr(&b.right);
                    self.edge(node, l);
                    self.edge(node, r);
                    node
                }
                klik_ast::Expr::Unary(u) => {
                    let node = self.new_node(&format!("un {:?}", u.op));
                    let c = self.walk_expr(&u.operand);
                    self.edge(node, c);
                    node
                }
                klik_ast::Expr::Call(c) => {
                    let node = self.new_node("call");
                    let callee = self.walk_expr(&c.callee);
                    self.edge(node, callee);
                    for arg in &c.args {
                        let a = self.walk_expr(arg);
                        self.edge(node, a);
                    }
                    node
                }
                klik_ast::Expr::MethodCall(m) => {
                    let node = self.new_node(&format!("method {}", m.method));
                    let recv = self.walk_expr(&m.receiver);
                    self.edge(node, recv);
                    for arg in &m.args {
                        let a = self.walk_expr(arg);
                        self.edge(node, a);
                    }
                    node
                }
                klik_ast::Expr::FieldAccess(f) => {
                    let node = self.new_node(&format!("field {}", f.field));
                    let o = self.walk_expr(&f.object);
                    self.edge(node, o);
                    node
                }
                klik_ast::Expr::If(i) => {
                    let node = self.new_node("if");
                    let c = self.walk_expr(&i.condition);
                    let t = self.walk_block(&i.then_block);
                    self.edge(node, c);
                    self.edge(node, t);
                    if let Some(e) = &i.else_block {
                        let eb = self.walk_expr(e);
                        self.edge(node, eb);
                    }
                    node
                }
                klik_ast::Expr::Match(m) => {
                    let node = self.new_node("match");
                    let s = self.walk_expr(&m.subject);
                    self.edge(node, s);
                    for arm in &m.arms {
                        let a = self.new_node("arm");
                        self.edge(node, a);
                        let body = self.walk_expr(&arm.body);
                        self.edge(a, body);
                    }
                    node
                }
                klik_ast::Expr::Block(b) => self.walk_block(b),
                klik_ast::Expr::Array(a) => {
                    let node = self.new_node("array");
                    for el in &a.elements {
                        let c = self.walk_expr(el);
                        self.edge(node, c);
                    }
                    node
                }
                klik_ast::Expr::Tuple(t) => {
                    let node = self.new_node("tuple");
                    for el in &t.elements {
                        let c = self.walk_expr(el);
                        self.edge(node, c);
                    }
                    node
                }
                klik_ast::Expr::StructInit(s) => {
                    let node = self.new_node(&format!("init {}", s.name));
                    for (name, val) in &s.fields {
                        let f = self.new_node(&format!("field {}", name));
                        let v = self.walk_expr(val);
                        self.edge(node, f);
                        self.edge(f, v);
                    }
                    node
                }
                klik_ast::Expr::Lambda(_) => self.new_node("lambda"),
                klik_ast::Expr::Await(a) => self.walk_expr(&a.expr),
                klik_ast::Expr::Range(_) => self.new_node("range"),
                klik_ast::Expr::Cast(c) => {
                    let node = self.new_node("cast");
                    let inner = self.walk_expr(&c.expr);
                    self.edge(node, inner);
                    node
                }
                klik_ast::Expr::Index(i) => {
                    let node = self.new_node("index");
                    let obj = self.walk_expr(&i.object);
                    let idx = self.walk_expr(&i.index);
                    self.edge(node, obj);
                    self.edge(node, idx);
                    node
                }
            }
        }
    }

    let mut builder = AstDotBuilder {
        next_id: 0,
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    let _ = builder.walk_program(program);

    let mut out = String::from("digraph AST {\n  rankdir=TB;\n");
    for n in builder.nodes {
        out.push_str(&n);
        out.push('\n');
    }
    for e in builder.edges {
        out.push_str(&e);
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

fn ir_to_dot(ir: &klik_ir::IrModule) -> String {
    let mut out = String::from("digraph IR {\n  rankdir=TB;\n  node [shape=box];\n");
    for func in &ir.functions {
        out.push_str(&format!(
            "  subgraph cluster_{} {{\n    label=\"fn {}\";\n",
            dot_escape(&func.name),
            dot_escape(&func.name)
        ));
        for (i, block) in func.blocks.iter().enumerate() {
            let mut label = format!("{}:\\l", block.label);
            for inst in &block.instructions {
                label.push_str(&format!("{:?}\\l", inst));
            }
            if let Some(term) = &block.terminator {
                label.push_str(&format!("{:?}\\l", term));
            }
            out.push_str(&format!(
                "    {}_b{} [label=\"{}\"];\n",
                dot_escape(&func.name),
                i,
                dot_escape(&label)
            ));
        }
        out.push_str("  }\n");
    }
    out.push_str("}\n");
    out
}

fn cfg_to_dot(ir: &klik_ir::IrModule) -> String {
    let mut out = String::from("digraph CFG {\n  rankdir=LR;\n  node [shape=ellipse];\n");
    for func in &ir.functions {
        out.push_str(&format!(
            "  subgraph cluster_cfg_{} {{\n    label=\"cfg {}\";\n",
            dot_escape(&func.name),
            dot_escape(&func.name)
        ));
        for (idx, block) in func.blocks.iter().enumerate() {
            out.push_str(&format!(
                "    {}_n{} [label=\"{}\"];\n",
                dot_escape(&func.name),
                idx,
                dot_escape(&block.label)
            ));
            if let Some(term) = &block.terminator {
                match term {
                    klik_ir::Terminator::Branch(klik_ir::BlockRef(t)) => {
                        out.push_str(&format!(
                            "    {}_n{} -> {}_n{};\n",
                            dot_escape(&func.name),
                            idx,
                            dot_escape(&func.name),
                            t
                        ));
                    }
                    klik_ir::Terminator::CondBranch(
                        _,
                        klik_ir::BlockRef(t),
                        klik_ir::BlockRef(e),
                    ) => {
                        out.push_str(&format!(
                            "    {}_n{} -> {}_n{} [label=\"true\"];\n",
                            dot_escape(&func.name),
                            idx,
                            dot_escape(&func.name),
                            t
                        ));
                        out.push_str(&format!(
                            "    {}_n{} -> {}_n{} [label=\"false\"];\n",
                            dot_escape(&func.name),
                            idx,
                            dot_escape(&func.name),
                            e
                        ));
                    }
                    klik_ir::Terminator::Switch(_, cases, klik_ir::BlockRef(default)) => {
                        for (c, klik_ir::BlockRef(t)) in cases {
                            out.push_str(&format!(
                                "    {}_n{} -> {}_n{} [label=\"{:?}\"];\n",
                                dot_escape(&func.name),
                                idx,
                                dot_escape(&func.name),
                                t,
                                c
                            ));
                        }
                        out.push_str(&format!(
                            "    {}_n{} -> {}_n{} [label=\"default\"];\n",
                            dot_escape(&func.name),
                            idx,
                            dot_escape(&func.name),
                            default
                        ));
                    }
                    _ => {}
                }
            }
        }
        out.push_str("  }\n");
    }
    out.push_str("}\n");
    out
}

fn render_dot_to_png(dot_path: &Path, png_path: &Path) -> Result<()> {
    let output = Command::new("dot")
        .arg("-Tpng")
        .arg(dot_path)
        .arg("-o")
        .arg(png_path)
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => bail!(
            "Graphviz failed for {}\nstdout:\n{}\nstderr:\n{}",
            dot_path.display(),
            String::from_utf8_lossy(&out.stdout).trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        ),
        Err(e) => bail!(
            "Failed to invoke Graphviz 'dot' for {}: {}",
            dot_path.display(),
            e
        ),
    }
}

fn open_in_browser(path: &Path) -> Result<()> {
    if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .status()
            .context("Failed to launch browser on Windows")?;
    } else if cfg!(target_os = "macos") {
        Command::new("open")
            .arg(path)
            .status()
            .context("Failed to launch browser on macOS")?;
    } else {
        Command::new("xdg-open")
            .arg(path)
            .status()
            .context("Failed to launch browser on Linux")?;
    }
    Ok(())
}

fn emit_debug_graphs(
    base_name: &str,
    frontend: &FrontendArtifacts,
    emit_ir: bool,
    emit_ast: bool,
    emit_cfg: bool,
) -> Result<()> {
    if emit_ast {
        let ast_dot_path = PathBuf::from(format!("{}.ast.dot", base_name));
        std::fs::write(&ast_dot_path, ast_to_dot(&frontend.ast))?;
        println!("{} {}", "AST DOT:".cyan().bold(), ast_dot_path.display());
    }

    if emit_ir {
        let ir = frontend
            .ir_module
            .as_ref()
            .context("--emit-ir requires IR generation")?;
        let ir_text_path = PathBuf::from(format!("{}.ir", base_name));
        let ir_dot_path = PathBuf::from(format!("{}.dot", base_name));
        std::fs::write(&ir_text_path, format!("{:#?}", ir))?;
        std::fs::write(&ir_dot_path, ir_to_dot(ir))?;
        println!("{} {}", "IR dump:".cyan().bold(), ir_text_path.display());
        println!("{} {}", "IR DOT:".cyan().bold(), ir_dot_path.display());
    }

    if emit_cfg {
        let ir = frontend
            .ir_module
            .as_ref()
            .context("--emit-cfg requires IR generation")?;
        let cfg_dot_path = PathBuf::from(format!("{}.cfg.dot", base_name));
        std::fs::write(&cfg_dot_path, cfg_to_dot(ir))?;
        println!("{} {}", "CFG DOT:".cyan().bold(), cfg_dot_path.display());
    }

    Ok(())
}

fn ensure_print_runtime_object(compiler: &str, out_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(out_dir)?;

    let src_path = out_dir.join("klik_print_runtime.c");
    let obj_ext = if cfg!(windows) { "obj" } else { "o" };
    let obj_path = out_dir.join(format!("klik_print_runtime.{}", obj_ext));

    let c_source = r#"
#include <stdio.h>
#include <stdint.h>

#if defined(_WIN32) && defined(__x86_64__)
#define KLIK_ABI __attribute__((ms_abi))
#else
#define KLIK_ABI
#endif

int KLIK_ABI klik_print_s(const char* s) {
    return printf("%s", s ? s : "");
}

int KLIK_ABI klik_print_i64(const void* v) {
    long long value = (long long)(intptr_t)v;
    char buf[32];
    char* p = &buf[31];
    unsigned long long n;
    *p = '\0';

    if (value < 0) {
        n = (unsigned long long)(-(value + 1)) + 1ULL;
    } else {
        n = (unsigned long long)value;
    }

    do {
        *--p = (char)('0' + (n % 10ULL));
        n /= 10ULL;
    } while (n != 0ULL);

    if (value < 0) {
        *--p = '-';
    }

    return fputs(p, stdout);
}
"#;
    std::fs::write(&src_path, c_source)?;

    let out = Command::new(compiler)
        .arg("-c")
        .arg(&src_path)
        .arg("-o")
        .arg(&obj_path)
        .output();

    match out {
        Ok(output) => {
            if output.status.success() {
                Ok(obj_path)
            } else {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!(
                    "Failed to compile runtime print wrapper with {}\nstdout:\n{}\nstderr:\n{}",
                    compiler,
                    stdout.trim(),
                    stderr.trim()
                )
            }
        }
        Err(e) => bail!("Failed to start compiler {}: {}", compiler, e),
    }
}

fn link_executable(obj_path: &Path, exe_path: &Path) -> Result<()> {
    let mut attempts: Vec<String> = Vec::new();

    let artifact_dir = obj_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let try_link = |attempts: &mut Vec<String>, tool: &str, args: Vec<OsString>| -> Result<bool> {
        let mut cmd = Command::new(tool);
        cmd.args(&args);
        match cmd.output() {
            Ok(out) => {
                if out.status.success() {
                    return Ok(true);
                }

                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                attempts.push(format!(
                    "{} failed (status {:?})\nstdout:\n{}\nstderr:\n{}",
                    tool,
                    out.status.code(),
                    stdout.trim(),
                    stderr.trim()
                ));
                Ok(false)
            }
            Err(err) => {
                attempts.push(format!("{} failed to start: {}", tool, err));
                Ok(false)
            }
        }
    };

    let obj = obj_path.as_os_str().to_os_string();
    let exe = exe_path.as_os_str().to_os_string();

    for compiler in ["clang", "gcc"] {
        match ensure_print_runtime_object(compiler, &artifact_dir) {
            Ok(runtime_obj) => {
                if try_link(
                    &mut attempts,
                    compiler,
                    vec![
                        obj.clone(),
                        runtime_obj.as_os_str().to_os_string(),
                        OsString::from("-o"),
                        exe.clone(),
                    ],
                )? {
                    return Ok(());
                }
            }
            Err(err) => attempts.push(format!(
                "{} runtime-wrapper compile failed: {:#}",
                compiler, err
            )),
        }
    }

    if cfg!(windows) {
        let out_arg = OsString::from(format!("/OUT:{}", exe_path.display()));
        if try_link(&mut attempts, "link.exe", vec![obj, out_arg])? {
            return Ok(());
        }
    }

    bail!(
        "Link step failed for {}\nTried clang, gcc{}\n{}",
        exe_path.display(),
        if cfg!(windows) { ", link.exe" } else { "" },
        attempts.join("\n\n")
    );
}

fn compile_native_executable(
    ast: &klik_ast::Program,
    output_path: &Path,
    release: bool,
) -> Result<()> {
    let rust_source = transpile_program_to_rust(ast)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let rust_file = std::env::temp_dir().join(format!(
        "klik_generated_{}_{}.rs",
        std::process::id(),
        stamp
    ));

    std::fs::write(&rust_file, rust_source)?;

    let mut cmd = Command::new("rustc");
    if release {
        cmd.arg("-O");
    }
    let status = cmd
        .arg(&rust_file)
        .arg("-o")
        .arg(output_path)
        .status()
        .context("Failed to invoke rustc for native executable emission")?;

    if !status.success() {
        bail!(
            "native linking step failed (rustc exited with status {:?})",
            status.code()
        );
    }

    let _ = std::fs::remove_file(&rust_file);
    Ok(())
}

fn transpile_program_to_rust(program: &klik_ast::Program) -> Result<String> {
    let mut out = String::new();
    out.push_str("// Generated by klik from KLIK source\n");
    out.push_str(
        "#![allow(unused_variables, unused_mut, dead_code, unused_imports, unused_parens)]\n",
    );
    out.push_str("use std::collections::HashMap;\n\n");

    // Helper function for display formatting
    out.push_str("fn klik_display<T: std::fmt::Debug>(val: &T) -> String {\n");
    out.push_str("    // Use Debug formatting but strip quotes from strings\n");
    out.push_str("    let s = format!(\"{:?}\", val);\n");
    out.push_str("    if s.starts_with('\"') && s.ends_with('\"') {\n");
    out.push_str("        s[1..s.len()-1].replace(\"\\\\n\", \"\\n\").replace(\"\\\\\\\"\", \"\\\"\").to_string()\n");
    out.push_str("    } else {\n");
    out.push_str("        s\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    for module in &program.modules {
        for item in &module.items {
            out.push_str(&transpile_item(item)?);
            out.push('\n');
        }
    }

    Ok(out)
}

fn transpile_item(item: &klik_ast::Item) -> Result<String> {
    match item {
        klik_ast::Item::Function(f) => transpile_function(f),
        klik_ast::Item::Const(c) => transpile_const(c),
        klik_ast::Item::Struct(s) => transpile_struct(s),
        klik_ast::Item::Enum(e) => transpile_enum(e),
        klik_ast::Item::Impl(imp) => transpile_impl(imp),
        klik_ast::Item::Trait(t) => transpile_trait(t),
        klik_ast::Item::TypeAlias(ta) => transpile_type_alias(ta),
        klik_ast::Item::Import(_) | klik_ast::Item::Module(_) | klik_ast::Item::Test(_) => {
            Ok(String::new())
        }
    }
}

fn transpile_struct(s: &klik_ast::StructDef) -> Result<String> {
    let mut out = String::new();
    out.push_str("#[derive(Debug, Clone)]\n");
    if s.is_pub {
        out.push_str("pub ");
    }
    out.push_str("struct ");
    out.push_str(&s.name);
    transpile_generic_params(&s.generic_params, &mut out);
    out.push_str(" {\n");
    for field in &s.fields {
        out.push_str("    ");
        if field.is_pub {
            out.push_str("pub ");
        }
        out.push_str(&field.name);
        out.push_str(": ");
        out.push_str(&transpile_type_expr(&field.type_expr)?);
        out.push_str(",\n");
    }
    out.push_str("}\n");
    Ok(out)
}

fn transpile_enum(e: &klik_ast::EnumDef) -> Result<String> {
    let mut out = String::new();
    out.push_str("#[derive(Debug, Clone)]\n");
    if e.is_pub {
        out.push_str("pub ");
    }
    out.push_str("enum ");
    out.push_str(&e.name);
    transpile_generic_params(&e.generic_params, &mut out);
    out.push_str(" {\n");
    for variant in &e.variants {
        out.push_str("    ");
        out.push_str(&variant.name);
        if !variant.fields.is_empty() {
            out.push('(');
            for (i, f) in variant.fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&transpile_type_expr(f)?);
            }
            out.push(')');
        }
        out.push_str(",\n");
    }
    out.push_str("}\n");
    Ok(out)
}

fn transpile_impl(imp: &klik_ast::ImplBlock) -> Result<String> {
    let mut out = String::new();
    out.push_str("impl");
    transpile_generic_params(&imp.generic_params, &mut out);
    out.push(' ');
    if let Some(trait_name) = &imp.trait_name {
        out.push_str(trait_name);
        out.push_str(" for ");
    }
    out.push_str(&imp.type_name);
    out.push_str(" {\n");
    for method in &imp.methods {
        let method_code = transpile_function(method)?;
        for line in method_code.lines() {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("}\n");
    Ok(out)
}

fn transpile_trait(t: &klik_ast::TraitDef) -> Result<String> {
    let mut out = String::new();
    if t.is_pub {
        out.push_str("pub ");
    }
    out.push_str("trait ");
    out.push_str(&t.name);
    transpile_generic_params(&t.generic_params, &mut out);
    out.push_str(" {\n");
    for method in &t.methods {
        out.push_str("    fn ");
        out.push_str(&method.name);
        out.push('(');
        for (i, p) in method.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            if p.name == "self" {
                out.push_str("&self");
            } else {
                out.push_str(&p.name);
                out.push_str(": ");
                out.push_str(&transpile_type_expr(&p.type_expr)?);
            }
        }
        out.push(')');
        if let Some(ret) = &method.return_type {
            let ret_ty = transpile_type_expr(ret)?;
            if ret_ty != "()" {
                out.push_str(" -> ");
                out.push_str(&ret_ty);
            }
        }
        if let Some(body) = &method.default_body {
            out.push_str(" {\n");
            out.push_str(&transpile_block_statements(body, 2, true)?);
            out.push_str("    }\n");
        } else {
            out.push_str(";\n");
        }
    }
    out.push_str("}\n");
    Ok(out)
}

fn transpile_type_alias(ta: &klik_ast::TypeAlias) -> Result<String> {
    let mut out = String::new();
    if ta.is_pub {
        out.push_str("pub ");
    }
    out.push_str("type ");
    out.push_str(&ta.name);
    transpile_generic_params(&ta.generic_params, &mut out);
    out.push_str(" = ");
    out.push_str(&transpile_type_expr(&ta.type_expr)?);
    out.push_str(";\n");
    Ok(out)
}

fn transpile_generic_params(params: &[klik_ast::GenericParam], out: &mut String) {
    if params.is_empty() {
        return;
    }
    out.push('<');
    for (i, gp) in params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&gp.name);
        if !gp.bounds.is_empty() {
            out.push_str(": ");
            for (j, b) in gp.bounds.iter().enumerate() {
                if j > 0 {
                    out.push_str(" + ");
                }
                if let klik_ast::TypeExpr::Named { name, .. } = b {
                    out.push_str(name);
                }
            }
        }
    }
    out.push('>');
}

fn transpile_const(c: &klik_ast::ConstDecl) -> Result<String> {
    let mut s = String::new();
    s.push_str("const ");
    s.push_str(&c.name);
    if let Some(ty) = &c.type_expr {
        s.push_str(": ");
        s.push_str(&transpile_type_expr(ty)?);
    }
    s.push_str(" = ");
    s.push_str(&transpile_expr(&c.value)?);
    s.push_str(";\n");
    Ok(s)
}

fn transpile_function(func: &klik_ast::Function) -> Result<String> {
    let mut s = String::new();
    s.push_str("fn ");
    s.push_str(&func.name);
    transpile_generic_params(&func.generic_params, &mut s);
    s.push('(');

    for (i, param) in func.params.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        if param.name == "self" {
            s.push_str("&self");
        } else {
            s.push_str(&param.name);
            s.push_str(": ");
            s.push_str(&transpile_type_expr(&param.type_expr)?);
        }
    }
    s.push(')');

    if let Some(ret) = &func.return_type {
        let ret_ty = transpile_type_expr(ret)?;
        if ret_ty != "()" {
            s.push_str(" -> ");
            s.push_str(&ret_ty);
        }
    }

    s.push_str(" {\n");
    s.push_str(&transpile_block_statements(&func.body, 1, true)?);
    s.push_str("}\n");
    Ok(s)
}

fn transpile_type_expr(ty: &klik_ast::TypeExpr) -> Result<String> {
    match ty {
        klik_ast::TypeExpr::Named {
            name, generic_args, ..
        } => {
            let mapped = match name.as_str() {
                "int" | "i64" => "i64",
                "i32" => "i32",
                "i16" => "i16",
                "i8" => "i8",
                "u64" | "uint" => "u64",
                "u32" => "u32",
                "u16" => "u16",
                "u8" => "u8",
                "f64" => "f64",
                "f32" => "f32",
                "bool" => "bool",
                "string" => "String",
                "char" => "char",
                "void" => "()",
                "Vec" => "Vec",
                "Map" => "HashMap",
                "Set" => "std::collections::HashSet",
                "Result" => "Result",
                "Option" => "Option",
                other => other,
            };
            if !generic_args.is_empty() {
                let args: Result<Vec<String>> =
                    generic_args.iter().map(transpile_type_expr).collect();
                Ok(format!("{}<{}>", mapped, args?.join(", ")))
            } else {
                Ok(mapped.to_string())
            }
        }
        klik_ast::TypeExpr::Array { element, .. } => {
            Ok(format!("Vec<{}>", transpile_type_expr(element)?))
        }
        klik_ast::TypeExpr::Optional { inner, .. } => {
            Ok(format!("Option<{}>", transpile_type_expr(inner)?))
        }
        klik_ast::TypeExpr::Tuple { elements, .. } => {
            let parts: Result<Vec<String>> = elements.iter().map(transpile_type_expr).collect();
            Ok(format!("({})", parts?.join(", ")))
        }
        klik_ast::TypeExpr::Function {
            params,
            return_type,
            ..
        } => {
            let params: Result<Vec<String>> = params.iter().map(transpile_type_expr).collect();
            Ok(format!(
                "fn({}) -> {}",
                params?.join(", "),
                transpile_type_expr(return_type)?
            ))
        }
        klik_ast::TypeExpr::Reference { inner, mutable, .. } => {
            if *mutable {
                Ok(format!("&mut {}", transpile_type_expr(inner)?))
            } else {
                Ok(format!("&{}", transpile_type_expr(inner)?))
            }
        }
    }
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn transpile_block_expr(block: &klik_ast::Block, level: usize) -> Result<String> {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&transpile_block_statements(block, level + 1, true)?);
    s.push_str(&format!("{}}}", indent(level)));
    Ok(s)
}

fn transpile_block_statements(
    block: &klik_ast::Block,
    level: usize,
    allow_tail_expr: bool,
) -> Result<String> {
    let mut out = String::new();
    for (i, stmt) in block.stmts.iter().enumerate() {
        let is_tail = allow_tail_expr
            && i + 1 == block.stmts.len()
            && matches!(stmt, klik_ast::Stmt::Expr(_));
        out.push_str(&transpile_stmt(stmt, level, is_tail)?);
        out.push('\n');
    }
    Ok(out)
}

fn transpile_stmt(stmt: &klik_ast::Stmt, level: usize, is_tail_expr: bool) -> Result<String> {
    let pad = indent(level);
    match stmt {
        klik_ast::Stmt::Let(s) => {
            let mut line = String::new();
            line.push_str(&pad);
            line.push_str("let ");
            if s.mutable {
                line.push_str("mut ");
            }
            line.push_str(&s.name);
            if let Some(ty) = &s.type_expr {
                line.push_str(": ");
                line.push_str(&transpile_type_expr(ty)?);
            }
            if let Some(value) = &s.value {
                line.push_str(" = ");
                line.push_str(&transpile_expr(value)?);
            }
            line.push(';');
            Ok(line)
        }
        klik_ast::Stmt::Expr(e) => {
            if is_tail_expr {
                Ok(format!("{}{}", pad, transpile_expr(e)?))
            } else {
                Ok(format!("{}{};", pad, transpile_expr(e)?))
            }
        }
        klik_ast::Stmt::Return(r) => {
            if let Some(v) = &r.value {
                Ok(format!("{}return {};", pad, transpile_expr(v)?))
            } else {
                Ok(format!("{}return;", pad))
            }
        }
        klik_ast::Stmt::Assign(a) => {
            if let Some(op) = &a.op {
                let compound = match op {
                    klik_ast::BinaryOp::Add => "+=",
                    klik_ast::BinaryOp::Sub => "-=",
                    klik_ast::BinaryOp::Mul => "*=",
                    klik_ast::BinaryOp::Div => "/=",
                    klik_ast::BinaryOp::Mod => "%=",
                    _ => "=",
                };
                Ok(format!(
                    "{}{} {} {};",
                    pad,
                    transpile_expr(&a.target)?,
                    compound,
                    transpile_expr(&a.value)?
                ))
            } else {
                Ok(format!(
                    "{}{} = {};",
                    pad,
                    transpile_expr(&a.target)?,
                    transpile_expr(&a.value)?
                ))
            }
        }
        klik_ast::Stmt::While(w) => {
            let mut line = String::new();
            line.push_str(&format!(
                "{}while {} {{\n",
                pad,
                transpile_expr(&w.condition)?
            ));
            line.push_str(&transpile_block_statements(&w.body, level + 1, false)?);
            line.push_str(&format!("{}}}", pad));
            Ok(line)
        }
        klik_ast::Stmt::For(f) => {
            let mut line = String::new();
            line.push_str(&format!(
                "{}for {} in {} {{\n",
                pad,
                f.variable,
                transpile_expr(&f.iterator)?
            ));
            line.push_str(&transpile_block_statements(&f.body, level + 1, false)?);
            line.push_str(&format!("{}}}", pad));
            Ok(line)
        }
        klik_ast::Stmt::Break(_) => Ok(format!("{}break;", pad)),
        klik_ast::Stmt::Continue(_) => Ok(format!("{}continue;", pad)),
        klik_ast::Stmt::Item(item) => {
            let code = transpile_item(item)?;
            let mut result = String::new();
            for line in code.lines() {
                result.push_str(&pad);
                result.push_str(line);
                result.push('\n');
            }
            Ok(result.trim_end().to_string())
        }
    }
}

fn transpile_expr(expr: &klik_ast::Expr) -> Result<String> {
    match expr {
        klik_ast::Expr::Literal(lit) => match &lit.kind {
            klik_ast::LiteralKind::Int(v) => Ok(format!("{}i64", v)),
            klik_ast::LiteralKind::Float(v) => {
                let s = v.to_string();
                if s.contains('.') {
                    Ok(format!("{}f64", s))
                } else {
                    Ok(format!("{}.0f64", s))
                }
            }
            klik_ast::LiteralKind::String(s) => Ok(format!("{:?}.to_string()", s)),
            klik_ast::LiteralKind::Bool(v) => Ok(v.to_string()),
            klik_ast::LiteralKind::Char(c) => Ok(format!("{:?}", c)),
            klik_ast::LiteralKind::None => Ok("None".to_string()),
        },
        klik_ast::Expr::Identifier(ident) => Ok(ident.name.clone()),
        klik_ast::Expr::Binary(bin) => {
            if bin.op == klik_ast::BinaryOp::Pipe {
                // Pipe operator:  left |> right  =>  right(left)
                // right can be a function call like map(|x| x*2) => we inject left as first arg
                return transpile_pipe(&bin.left, &bin.right);
            }
            let op = match bin.op {
                klik_ast::BinaryOp::Add => "+",
                klik_ast::BinaryOp::Sub => "-",
                klik_ast::BinaryOp::Mul => "*",
                klik_ast::BinaryOp::Div => "/",
                klik_ast::BinaryOp::Mod => "%",
                klik_ast::BinaryOp::Eq => "==",
                klik_ast::BinaryOp::Neq => "!=",
                klik_ast::BinaryOp::Lt => "<",
                klik_ast::BinaryOp::Gt => ">",
                klik_ast::BinaryOp::Lte => "<=",
                klik_ast::BinaryOp::Gte => ">=",
                klik_ast::BinaryOp::And => "&&",
                klik_ast::BinaryOp::Or => "||",
                klik_ast::BinaryOp::BitAnd => "&",
                klik_ast::BinaryOp::BitOr => "|",
                klik_ast::BinaryOp::BitXor => "^",
                klik_ast::BinaryOp::Shl => "<<",
                klik_ast::BinaryOp::Shr => ">>",
                klik_ast::BinaryOp::Pipe => unreachable!(),
            };
            let left = transpile_expr(&bin.left)?;
            let right = transpile_expr(&bin.right)?;
            // For + operator, detect if either side is likely a string and use format! to avoid &str vs String issues
            if op == "+" {
                fn is_definitely_numeric(expr: &klik_ast::Expr) -> bool {
                    match expr {
                        klik_ast::Expr::Literal(lit) => matches!(
                            lit.kind,
                            klik_ast::LiteralKind::Int(_) | klik_ast::LiteralKind::Float(_)
                        ),
                        klik_ast::Expr::Unary(un) => is_definitely_numeric(&un.operand),
                        klik_ast::Expr::Binary(bin) => {
                            // Sub/Mul/Div/Mod are always numeric
                            if matches!(
                                bin.op,
                                klik_ast::BinaryOp::Sub
                                    | klik_ast::BinaryOp::Mul
                                    | klik_ast::BinaryOp::Div
                                    | klik_ast::BinaryOp::Mod
                                    | klik_ast::BinaryOp::Shl
                                    | klik_ast::BinaryOp::Shr
                                    | klik_ast::BinaryOp::BitAnd
                                    | klik_ast::BinaryOp::BitOr
                                    | klik_ast::BinaryOp::BitXor
                            ) {
                                return true;
                            }
                            // Add of two definitely-numeric things is numeric
                            if bin.op == klik_ast::BinaryOp::Add {
                                return is_definitely_numeric(&bin.left)
                                    && is_definitely_numeric(&bin.right);
                            }
                            false
                        }
                        klik_ast::Expr::Index(_) => true, // array[i] is typically numeric
                        klik_ast::Expr::Call(call) => {
                            // count_xxx / len etc return numeric
                            if let klik_ast::Expr::Identifier(id) = call.callee.as_ref() {
                                matches!(id.name.as_str(), "len" | "count" | "abs")
                            } else {
                                false
                            }
                        }
                        _ => false,
                    }
                }
                fn is_definitely_string(expr: &klik_ast::Expr) -> bool {
                    match expr {
                        klik_ast::Expr::Literal(lit) => {
                            matches!(lit.kind, klik_ast::LiteralKind::String(_))
                        }
                        klik_ast::Expr::Binary(bin) if bin.op == klik_ast::BinaryOp::Add => {
                            is_definitely_string(&bin.left) || is_definitely_string(&bin.right)
                        }
                        _ => false,
                    }
                }
                if is_definitely_string(&bin.left) || is_definitely_string(&bin.right) {
                    return Ok(format!("format!(\"{{}}{{}}\", {}, {})", left, right));
                }
            }
            Ok(format!("({} {} {})", left, op, right))
        }
        klik_ast::Expr::Unary(un) => {
            let op = match un.op {
                klik_ast::UnaryOp::Neg => "-",
                klik_ast::UnaryOp::Not => "!",
                klik_ast::UnaryOp::BitNot => "!",
                klik_ast::UnaryOp::Ref => "&",
                klik_ast::UnaryOp::RefMut => "&mut ",
                klik_ast::UnaryOp::Deref => "*",
            };
            Ok(format!("({}{})", op, transpile_expr(&un.operand)?))
        }
        klik_ast::Expr::Call(call) => {
            let args: Result<Vec<String>> = call.args.iter().map(transpile_expr).collect();
            let args = args?;

            if let klik_ast::Expr::Identifier(ident) = call.callee.as_ref() {
                match ident.name.as_str() {
                    "println" => {
                        if args.is_empty() {
                            return Ok("println!()".to_string());
                        }
                        // Use a helper to format each argument
                        let fmt_args: Vec<String> = args
                            .iter()
                            .map(|a| format!("klik_display(&{})", a))
                            .collect();
                        let fmt = std::iter::repeat_n("{}", fmt_args.len())
                            .collect::<Vec<_>>()
                            .join(" ");
                        return Ok(format!("println!(\"{}\", {})", fmt, fmt_args.join(", ")));
                    }
                    "print" => {
                        if args.is_empty() {
                            return Ok("print!(\"\")".to_string());
                        }
                        let fmt_args: Vec<String> = args
                            .iter()
                            .map(|a| format!("klik_display(&{})", a))
                            .collect();
                        let fmt = std::iter::repeat_n("{}", fmt_args.len())
                            .collect::<Vec<_>>()
                            .join(" ");
                        return Ok(format!("print!(\"{}\", {})", fmt, fmt_args.join(", ")));
                    }
                    "to_string" if args.len() == 1 => {
                        return Ok(format!("({}).to_string()", args[0]));
                    }
                    "len" if args.len() == 1 => {
                        return Ok(format!("({}).len() as i64", args[0]));
                    }
                    "assert" if args.len() == 1 => {
                        return Ok(format!("assert!({})", args[0]));
                    }
                    "spawn" if args.len() == 1 => {
                        return Ok(format!("std::thread::spawn(move || {{ {} }})", args[0]));
                    }
                    "Some" if args.len() == 1 => {
                        return Ok(format!("Some({})", args[0]));
                    }
                    "Ok" if args.len() == 1 => {
                        return Ok(format!("Ok({})", args[0]));
                    }
                    "Err" if args.len() == 1 => {
                        return Ok(format!("Err({})", args[0]));
                    }
                    // Iterator functions used as standalone (e.g. in pipes)
                    "map" | "filter" | "fold" | "for_each" | "flat_map" | "take" | "skip"
                    | "zip" | "enumerate" | "any" | "all" | "find" | "position" | "count"
                    | "collect" | "sum" | "min" | "max" | "reduce" => {
                        // These are handled when used in pipe context
                        return Ok(format!("{}({})", ident.name, args.join(", ")));
                    }
                    _ => {}
                }
            }

            // Auto-clone identifier arguments to avoid move issues (KLIK has value semantics)
            let cloned_args: Vec<String> = call
                .args
                .iter()
                .zip(args.iter())
                .map(|(expr, transpiled)| {
                    if matches!(expr, klik_ast::Expr::Identifier(_)) {
                        format!("{}.clone()", transpiled)
                    } else {
                        transpiled.clone()
                    }
                })
                .collect();
            Ok(format!(
                "{}({})",
                transpile_expr(&call.callee)?,
                cloned_args.join(", ")
            ))
        }
        klik_ast::Expr::MethodCall(mc) => {
            let receiver = transpile_expr(&mc.receiver)?;
            let args: Result<Vec<String>> = mc.args.iter().map(transpile_expr).collect();
            let args = args?;
            Ok(format!("{}.{}({})", receiver, mc.method, args.join(", ")))
        }
        klik_ast::Expr::FieldAccess(fa) => {
            let obj = transpile_expr(&fa.object)?;
            Ok(format!("{}.{}", obj, fa.field))
        }
        klik_ast::Expr::Index(idx) => {
            let obj = transpile_expr(&idx.object)?;
            let index = transpile_expr(&idx.index)?;
            Ok(format!("{}[{} as usize]", obj, index))
        }
        klik_ast::Expr::If(if_expr) => {
            let mut s = String::new();
            s.push_str(&format!(
                "if {} {}",
                transpile_expr(&if_expr.condition)?,
                transpile_block_expr(&if_expr.then_block, 0)?
            ));
            if let Some(else_expr) = &if_expr.else_block {
                s.push_str(" else ");
                s.push_str(&transpile_expr(else_expr)?);
            }
            Ok(s)
        }
        klik_ast::Expr::Match(m) => {
            let mut s = String::new();
            s.push_str(&format!("match {} {{\n", transpile_expr(&m.subject)?));
            for arm in &m.arms {
                s.push_str(&format!(
                    "    {} => {},\n",
                    transpile_pattern(&arm.pattern)?,
                    transpile_expr(&arm.body)?
                ));
            }
            s.push('}');
            Ok(s)
        }
        klik_ast::Expr::Block(block) => transpile_block_expr(block, 0),
        klik_ast::Expr::Array(arr) => {
            let elems: Result<Vec<String>> = arr.elements.iter().map(transpile_expr).collect();
            Ok(format!("vec![{}]", elems?.join(", ")))
        }
        klik_ast::Expr::Tuple(tup) => {
            let elems: Result<Vec<String>> = tup.elements.iter().map(transpile_expr).collect();
            Ok(format!("({})", elems?.join(", ")))
        }
        klik_ast::Expr::StructInit(si) => {
            let mut s = String::new();
            s.push_str(&si.name);
            s.push_str(" { ");
            for (i, (name, val)) in si.fields.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(name);
                s.push_str(": ");
                s.push_str(&transpile_expr(val)?);
            }
            s.push_str(" }");
            Ok(s)
        }
        klik_ast::Expr::Lambda(l) => {
            let mut s = String::new();
            s.push('|');
            for (i, param) in l.params.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&param.name);
                // Only add type annotation for non-inferred types
                if !matches!(
                    &param.type_expr,
                    klik_ast::TypeExpr::Named { name, .. } if name == "_" || name == "void"
                ) {
                    let ty = transpile_type_expr(&param.type_expr)?;
                    if ty != "()" {
                        s.push_str(": ");
                        s.push_str(&ty);
                    }
                }
            }
            s.push_str("| ");
            s.push_str(&transpile_expr(&l.body)?);
            Ok(s)
        }
        klik_ast::Expr::Await(a) => {
            let inner = transpile_expr(&a.expr)?;
            Ok(format!("{}.await", inner))
        }
        klik_ast::Expr::Range(r) => {
            let start = match &r.start {
                Some(e) => transpile_expr(e)?,
                None => "".to_string(),
            };
            let end = match &r.end {
                Some(e) => transpile_expr(e)?,
                None => "".to_string(),
            };
            if r.inclusive {
                Ok(format!("({}..={})", start, end))
            } else {
                Ok(format!("({}..{})", start, end))
            }
        }
        klik_ast::Expr::Cast(c) => {
            let inner = transpile_expr(&c.expr)?;
            let ty = transpile_type_expr(&c.type_expr)?;
            Ok(format!("({} as {})", inner, ty))
        }
    }
}

/// Transpile pipe operator: left |> right
/// Handles patterns like:
///   arr |> map(|x| x*2)  =>  arr.into_iter().map(|x| x*2)
///   arr |> filter(|x| x>5) =>  .filter(|x| x>5)
///   arr |> sum()  =>  .sum::<i64>()
///   arr |> collect()  =>  .collect::<Vec<_>>()
fn transpile_pipe(left: &klik_ast::Expr, right: &klik_ast::Expr) -> Result<String> {
    let left_code = transpile_expr(left)?;

    // Check if left is already a pipe result (contains .into_iter())
    let is_chained = left_code.contains(".into_iter()") || left_code.contains(".iter()");

    // Clone identifiers to avoid move issues (KLIK has value semantics)
    let left_start = if !is_chained && matches!(left, klik_ast::Expr::Identifier(_)) {
        format!("{}.clone()", left_code)
    } else {
        left_code.clone()
    };

    match right {
        klik_ast::Expr::Call(call) => {
            if let klik_ast::Expr::Identifier(ident) = call.callee.as_ref() {
                let args: Result<Vec<String>> = call.args.iter().map(transpile_expr).collect();
                let args = args?;
                let method = &ident.name;

                match method.as_str() {
                    "map" | "flat_map" | "for_each" | "inspect" => {
                        let iter_part = if is_chained {
                            format!("{}", left_code)
                        } else {
                            format!("{}.into_iter()", left_start)
                        };
                        Ok(format!("{}.{}({})", iter_part, method, args.join(", ")))
                    }
                    "filter" | "take_while" | "skip_while" | "any" | "all" | "find"
                    | "position" => {
                        // These methods pass references to the closure.
                        // Wrap lambda to auto-deref: |x| pred(x) -> |x| pred(*x)
                        let iter_part = if is_chained {
                            format!("{}", left_code)
                        } else {
                            format!("{}.into_iter()", left_start)
                        };
                        // If the arg is a lambda, adjust it to accept a reference
                        let adjusted_args: Vec<String> = call
                            .args
                            .iter()
                            .map(|arg| {
                                if let klik_ast::Expr::Lambda(l) = arg {
                                    // Generate lambda that takes a reference parameter
                                    let params: Vec<String> =
                                        l.params.iter().map(|p| p.name.clone()).collect();
                                    let body = transpile_expr(&l.body).unwrap_or_default();
                                    format!(
                                        "|{}| {{ let {} = {}; {} }}",
                                        params.join(", "),
                                        params
                                            .iter()
                                            .map(|p| format!("{}", p))
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                        params
                                            .iter()
                                            .map(|p| format!("*{}", p))
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                        body
                                    )
                                } else {
                                    transpile_expr(arg).unwrap_or_default()
                                }
                            })
                            .collect();
                        Ok(format!(
                            "{}.{}({})",
                            iter_part,
                            method,
                            adjusted_args.join(", ")
                        ))
                    }
                    "take" | "skip" | "enumerate" | "zip" => {
                        let iter_part = if is_chained {
                            format!("{}", left_code)
                        } else {
                            format!("{}.into_iter()", left_start)
                        };
                        Ok(format!("{}.{}({})", iter_part, method, args.join(", ")))
                    }
                    "sum" => {
                        if is_chained {
                            Ok(format!("{}.sum::<i64>()", left_code))
                        } else {
                            Ok(format!("{}.into_iter().sum::<i64>()", left_start))
                        }
                    }
                    "count" => {
                        if is_chained {
                            Ok(format!("{}.count() as i64", left_code))
                        } else {
                            Ok(format!("{}.into_iter().count() as i64", left_start))
                        }
                    }
                    "collect" => {
                        if is_chained {
                            Ok(format!("{}.collect::<Vec<_>>()", left_code))
                        } else {
                            Ok(format!("{}.into_iter().collect::<Vec<_>>()", left_start))
                        }
                    }
                    "min" => {
                        if is_chained {
                            Ok(format!("{}.min().unwrap()", left_code))
                        } else {
                            Ok(format!("{}.into_iter().min().unwrap()", left_start))
                        }
                    }
                    "max" => {
                        if is_chained {
                            Ok(format!("{}.max().unwrap()", left_code))
                        } else {
                            Ok(format!("{}.into_iter().max().unwrap()", left_start))
                        }
                    }
                    "fold" => {
                        if is_chained {
                            Ok(format!("{}.fold({})", left_code, args.join(", ")))
                        } else {
                            Ok(format!(
                                "{}.into_iter().fold({})",
                                left_start,
                                args.join(", ")
                            ))
                        }
                    }
                    "reduce" => {
                        if is_chained {
                            Ok(format!(
                                "{}.reduce({}).unwrap()",
                                left_code,
                                args.join(", ")
                            ))
                        } else {
                            Ok(format!(
                                "{}.into_iter().reduce({}).unwrap()",
                                left_start,
                                args.join(", ")
                            ))
                        }
                    }
                    _ => {
                        // Generic function call: f(left, ...)
                        let mut all_args = vec![left_start];
                        all_args.extend(args);
                        Ok(format!("{}({})", method, all_args.join(", ")))
                    }
                }
            } else {
                let func_code = transpile_expr(&call.callee)?;
                Ok(format!("{}({})", func_code, left_start))
            }
        }
        klik_ast::Expr::Identifier(ident) => {
            // Simple function reference: x |> f  =>  f(x)
            Ok(format!("{}({})", ident.name, left_start))
        }
        _ => {
            let right_code = transpile_expr(right)?;
            Ok(format!("{}({})", right_code, left_start))
        }
    }
}

fn transpile_pattern(pattern: &klik_ast::Pattern) -> Result<String> {
    match pattern {
        klik_ast::Pattern::Literal(lit) => match &lit.kind {
            klik_ast::LiteralKind::Int(v) => Ok(format!("{}i64", v)),
            klik_ast::LiteralKind::Float(v) => Ok(v.to_string()),
            klik_ast::LiteralKind::String(s) => Ok(format!("{:?}", s)),
            klik_ast::LiteralKind::Bool(v) => Ok(v.to_string()),
            klik_ast::LiteralKind::Char(c) => Ok(format!("{:?}", c)),
            klik_ast::LiteralKind::None => Ok("None".to_string()),
        },
        klik_ast::Pattern::Identifier(name, _) => Ok(name.clone()),
        klik_ast::Pattern::Wildcard(_) => Ok("_".to_string()),
        klik_ast::Pattern::Tuple(pats, _) => {
            let parts: Result<Vec<String>> = pats.iter().map(transpile_pattern).collect();
            Ok(format!("({})", parts?.join(", ")))
        }
        klik_ast::Pattern::Struct { name, fields, .. } => {
            let mut s = format!("{} {{ ", name);
            for (i, (fname, pat)) in fields.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                let pat_str = transpile_pattern(pat)?;
                if pat_str == *fname {
                    s.push_str(fname);
                } else {
                    s.push_str(&format!("{}: {}", fname, pat_str));
                }
            }
            s.push_str(" }");
            Ok(s)
        }
        klik_ast::Pattern::Enum {
            name,
            variant,
            fields,
            ..
        } => {
            if fields.is_empty() {
                Ok(format!("{}::{}", name, variant))
            } else {
                let parts: Result<Vec<String>> = fields.iter().map(transpile_pattern).collect();
                Ok(format!("{}::{}({})", name, variant, parts?.join(", ")))
            }
        }
        klik_ast::Pattern::Or(pats, _) => {
            let parts: Result<Vec<String>> = pats.iter().map(transpile_pattern).collect();
            Ok(parts?.join(" | "))
        }
    }
}

/// Create a new KLIK project
pub fn new_project(name: &str, path: Option<PathBuf>) -> Result<()> {
    let base_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
    println!("{} new project `{}`", "Creating".green().bold(), name);

    klik_package_manager::PackageManager::init_project(&base_path, name)?;

    println!(
        "{} project `{}` created at {}",
        "Done".green().bold(),
        name,
        base_path.join(name).display()
    );
    Ok(())
}

/// Initialize a KLIK project (alias to `new`, with optional name)
pub fn init_project(name: Option<&str>, path: Option<PathBuf>) -> Result<()> {
    if let Some(name) = name {
        return new_project(name, path);
    }

    let cwd = std::env::current_dir()?;
    let inferred_name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .context("Could not infer project name from current directory")?
        .to_string();
    let parent = cwd
        .parent()
        .context("Could not determine parent directory for project initialization")?
        .to_path_buf();

    new_project(&inferred_name, Some(parent))
}

/// Build the project
pub fn build(
    input: Option<&Path>,
    release: bool,
    target: &str,
    output: Option<PathBuf>,
    emit_all: bool,
) -> Result<()> {
    build_with_options(
        input, release, target, output, emit_all, "O1", false, false, false, false,
    )
}

pub fn build_with_options(
    input: Option<&Path>,
    release: bool,
    target: &str,
    output: Option<PathBuf>,
    emit_all: bool,
    opt_level: &str,
    emit_ir: bool,
    emit_ast: bool,
    emit_cfg: bool,
    trace: bool,
) -> Result<()> {
    let opt_level = crate::pipeline::CliOptLevel::parse(opt_level)?;
    if let Some(input_file) = input {
        return build_single_file(
            input_file, release, target, output, emit_all, opt_level, emit_ir, emit_ast, emit_cfg,
            trace,
        );
    }

    build_project(
        release, target, output, emit_all, opt_level, emit_ir, emit_ast, emit_cfg, trace,
    )
}

fn build_project(
    release: bool,
    target: &str,
    output: Option<PathBuf>,
    emit_all: bool,
    opt_level: crate::pipeline::CliOptLevel,
    emit_ir: bool,
    emit_ast: bool,
    emit_cfg: bool,
    trace: bool,
) -> Result<()> {
    let timer = Instant::now();
    let cwd = std::env::current_dir()?;
    let mut pm = klik_package_manager::PackageManager::new(cwd.clone());
    let manifest = pm.load_manifest()?;
    let project_name = manifest.package.name.clone();

    println!(
        "{} {} v{} ({})",
        "Compiling".green().bold(),
        project_name,
        manifest.package.version,
        if release { "release" } else { "debug" }
    );

    let sources = pm.find_sources()?;
    if sources.is_empty() {
        bail!("No source files found in src/");
    }

    let entry_file = pm.entry_file();
    if !entry_file.exists() {
        bail!("Entry file not found: {}", entry_file.display());
    }

    let source_code = std::fs::read_to_string(&entry_file)
        .context(format!("Failed to read {}", entry_file.display()))?;
    let file_name = entry_file.to_string_lossy().to_string();
    let need_ir = target != "native" || emit_ir || emit_cfg || trace;
    let frontend = compile_frontend(
        &source_code,
        &file_name,
        &project_name,
        need_ir,
        opt_level,
        trace,
    )?;

    emit_debug_graphs(&project_name, &frontend, emit_ir, emit_ast, emit_cfg)?;

    let output_path = ensure_executable_path(
        target,
        output.unwrap_or_else(|| default_project_output_path(&cwd, release, &project_name)),
    );

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if target == "native" {
        trace_log(trace, "[CODEGEN] native transpiler backend selected");
        compile_native_executable(&frontend.ast, &output_path, release)?;
        trace_log(trace, "[LINK] rustc produced executable");
    } else {
        trace_log(trace, "[CODEGEN] Cranelift IR generated");
        let codegen_target = parse_target(target)?;
        let codegen = klik_codegen::CodeGenerator::new(codegen_target);
        let ir_module = frontend
            .ir_module
            .expect("IR module required for non-native targets");

        let artifact_dir = build_artifact_dir(&cwd);
        std::fs::create_dir_all(&artifact_dir)?;
        let obj_path = default_object_path(&cwd, &project_name);
        let ir_path = default_ir_artifact_path(&cwd, &project_name);
        let object_bytes = codegen.generate(&ir_module)?;
        std::fs::write(&obj_path, &object_bytes)?;

        if emit_all {
            std::fs::write(&ir_path, format!("{:#?}", ir_module))?;
            println!("{} {}", "IR: ".cyan().bold(), ir_path.display());
            println!("{} {}", "Object: ".cyan().bold(), obj_path.display());
        }

        trace_log(trace, "[LINK] linking executable");
        link_executable(&obj_path, &output_path).with_context(|| {
            format!(
                "Failed linking {} -> {}",
                obj_path.display(),
                output_path.display()
            )
        })?;
        trace_log(trace, "[LINK] executable linked");

        if !emit_all {
            let _ = std::fs::remove_file(&obj_path);
            let _ = std::fs::remove_file(&ir_path);
        }
    }

    let elapsed = timer.elapsed();
    println!(
        "{} {} in {:.2}s",
        "Finished".green().bold(),
        if release { "release" } else { "debug" },
        elapsed.as_secs_f64()
    );

    Ok(())
}

fn build_single_file(
    input: &Path,
    release: bool,
    target: &str,
    output: Option<PathBuf>,
    emit_all: bool,
    opt_level: crate::pipeline::CliOptLevel,
    emit_ir: bool,
    emit_ast: bool,
    emit_cfg: bool,
    trace: bool,
) -> Result<()> {
    let timer = Instant::now();
    let cwd = std::env::current_dir()?;

    if !input.exists() {
        bail!("Input file not found: {}", input.display());
    }
    if input.extension().and_then(|e| e.to_str()) != Some("klik") {
        bail!("Input file must have .klik extension: {}", input.display());
    }

    let module_name = input
        .file_stem()
        .and_then(|n| n.to_str())
        .context("Could not infer output name from input file")?
        .to_string();

    println!(
        "{} {} ({})",
        "Compiling".green().bold(),
        input.display(),
        if release { "release" } else { "debug" }
    );

    let source_code =
        std::fs::read_to_string(input).context(format!("Failed to read {}", input.display()))?;
    let file_name = input.to_string_lossy().to_string();

    let need_ir = target != "native" || emit_ir || emit_cfg || trace;
    let frontend = compile_frontend(
        &source_code,
        &file_name,
        &module_name,
        need_ir,
        opt_level,
        trace,
    )?;

    emit_debug_graphs(&module_name, &frontend, emit_ir, emit_ast, emit_cfg)?;

    let output_path = ensure_executable_path(
        target,
        output.unwrap_or_else(|| default_binary_path(&cwd, &module_name)),
    );
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if target == "native" {
        trace_log(trace, "[CODEGEN] native transpiler backend selected");
        compile_native_executable(&frontend.ast, &output_path, release)?;
        trace_log(trace, "[LINK] rustc produced executable");
    } else {
        trace_log(trace, "[CODEGEN] Cranelift IR generated");
        let codegen_target = parse_target(target)?;
        let codegen = klik_codegen::CodeGenerator::new(codegen_target);
        let ir_module = frontend
            .ir_module
            .expect("IR module required for non-native targets");

        let artifact_dir = build_artifact_dir(&cwd);
        std::fs::create_dir_all(&artifact_dir)?;
        let obj_path = default_object_path(&cwd, &module_name);
        let ir_path = default_ir_artifact_path(&cwd, &module_name);
        let object_bytes = codegen.generate(&ir_module)?;
        std::fs::write(&obj_path, &object_bytes)?;

        if emit_all {
            std::fs::write(&ir_path, format!("{:#?}", ir_module))?;
            println!("{} {}", "IR: ".cyan().bold(), ir_path.display());
            println!("{} {}", "Object: ".cyan().bold(), obj_path.display());
        }

        trace_log(trace, "[LINK] linking executable");
        link_executable(&obj_path, &output_path).with_context(|| {
            format!(
                "Failed linking {} -> {}",
                obj_path.display(),
                output_path.display()
            )
        })?;
        trace_log(trace, "[LINK] executable linked");

        if !emit_all {
            let _ = std::fs::remove_file(&obj_path);
            let _ = std::fs::remove_file(&ir_path);
        }
    }

    let elapsed = timer.elapsed();
    println!(
        "{} {} in {:.2}s",
        "Finished".green().bold(),
        if release { "release" } else { "debug" },
        elapsed.as_secs_f64()
    );
    println!("{} {}", "Output:".green().bold(), output_path.display());

    Ok(())
}

/// Run the project
pub fn run(input: Option<&Path>, release: bool, trace: bool, args: &[String]) -> Result<()> {
    let cwd = std::env::current_dir()?;

    let exe_path = if let Some(input_file) = input {
        let stem = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .context("Could not infer executable name from input file")?;
        let exe = default_binary_path(&cwd, stem);
        build_with_options(
            Some(input_file),
            release,
            "native",
            Some(exe.clone()),
            false,
            "O1",
            false,
            false,
            false,
            trace,
        )?;
        exe
    } else {
        let mut pm = klik_package_manager::PackageManager::new(cwd.clone());
        let manifest = pm.load_manifest()?;
        let exe = default_project_output_path(&cwd, release, &manifest.package.name);
        build_with_options(
            None,
            release,
            "native",
            Some(exe.clone()),
            false,
            "O1",
            false,
            false,
            false,
            trace,
        )?;
        exe
    };

    println!("{} `{}`", "Running".green().bold(), exe_path.display());

    let status = std::process::Command::new(&exe_path)
        .args(args)
        .status()
        .context("Failed to run program")?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    trace_log(trace, "[RUN] program executed successfully");
    Ok(())
}

pub fn visualize(input: &Path, open: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    if !input.exists() {
        bail!("Input file not found: {}", input.display());
    }
    if input.extension().and_then(|e| e.to_str()) != Some("klik") {
        bail!("Input file must have .klik extension: {}", input.display());
    }

    let module_name = input
        .file_stem()
        .and_then(|n| n.to_str())
        .context("Could not infer module name from input file")?
        .to_string();
    let source_code = std::fs::read_to_string(input)?;
    let file_name = input.to_string_lossy().to_string();

    let frontend = compile_frontend(
        &source_code,
        &file_name,
        &module_name,
        true,
        crate::pipeline::CliOptLevel::O1,
        false,
    )?;
    let ir = frontend
        .ir_module
        .as_ref()
        .context("IR module not available for visualization")?;

    let out_dir = cwd.join("visualization");
    std::fs::create_dir_all(&out_dir)?;

    let ast_dot = out_dir.join("AST.dot");
    let ast_png = out_dir.join("AST.png");
    let ir_dot = out_dir.join("IR.dot");
    let ir_png = out_dir.join("IR.png");
    let cfg_dot = out_dir.join("CFG.dot");
    let cfg_png = out_dir.join("CFG.png");
    let html_path = out_dir.join("pipeline.html");

    std::fs::write(&ast_dot, ast_to_dot(&frontend.ast))?;
    std::fs::write(&ir_dot, ir_to_dot(ir))?;
    std::fs::write(&cfg_dot, cfg_to_dot(ir))?;

    render_dot_to_png(&ast_dot, &ast_png)?;
    render_dot_to_png(&ir_dot, &ir_png)?;
    render_dot_to_png(&cfg_dot, &cfg_png)?;

    let html = r#"<!doctype html>
<html>
<head>
  <meta charset=\"utf-8\" />
  <title>KLIK Compiler Pipeline</title>
  <style>
    body { font-family: Segoe UI, Arial, sans-serif; margin: 20px; }
    h1, h2 { margin: 8px 0; }
    img { max-width: 100%; border: 1px solid #ddd; margin: 8px 0 20px 0; }
  </style>
</head>
<body>
  <h1>KLIK Compiler Visualization</h1>
  <h2>AST</h2>
  <img src=\"AST.png\" alt=\"AST graph\" />
  <h2>IR</h2>
  <img src=\"IR.png\" alt=\"IR graph\" />
  <h2>CFG</h2>
  <img src=\"CFG.png\" alt=\"CFG graph\" />
</body>
</html>
"#;
    std::fs::write(&html_path, html)?;

    println!("{} {}", "Generated:".green().bold(), out_dir.display());
    println!("  {}", ast_dot.display());
    println!("  {}", ast_png.display());
    println!("  {}", ir_dot.display());
    println!("  {}", ir_png.display());
    println!("  {}", cfg_dot.display());
    println!("  {}", cfg_png.display());
    println!("  {}", html_path.display());

    if open {
        open_in_browser(&html_path)?;
    }

    Ok(())
}

fn run_binary_capture(exe_path: &Path) -> Result<(String, f64)> {
    let start = Instant::now();
    let output = Command::new(exe_path)
        .output()
        .with_context(|| format!("Failed to run {}", exe_path.display()))?;
    let exec_secs = start.elapsed().as_secs_f64();

    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n");

    if !output.status.success() {
        bail!(
            "program exited with status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            stdout.trim(),
            stderr.trim()
        );
    }

    Ok((stdout, exec_secs))
}

struct BackendRunResult {
    output: String,
    compile_ms: u128,
    exec_secs: f64,
}

fn run_example_backend(
    cwd: &Path,
    source: &Path,
    target: &str,
    emit_all: bool,
) -> Result<BackendRunResult> {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Could not infer output name for backend test")?;
    let exe = default_binary_path(cwd, stem);

    if exe.exists() {
        let _ = std::fs::remove_file(&exe);
    }

    let compile_start = Instant::now();
    build_with_options(
        Some(source),
        false,
        target,
        Some(exe.clone()),
        emit_all,
        "O1",
        false,
        false,
        false,
        false,
    )?;
    let compile_ms = compile_start.elapsed().as_millis();

    if !exe.exists() {
        bail!("Expected executable not found: {}", exe.display());
    }

    let (output, exec_secs) = run_binary_capture(&exe)?;
    Ok(BackendRunResult {
        output,
        compile_ms,
        exec_secs,
    })
}

fn classify_backend_error(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("verifier") {
        "VERIFIER ERROR"
    } else if lower.contains("link step failed") || lower.contains("failed linking") {
        "RUNTIME ERROR"
    } else if lower.contains("failed to run") || lower.contains("not a valid application") {
        "RUNTIME ERROR"
    } else {
        "RUNTIME ERROR"
    }
}

pub fn test_backend() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let cranelift_target = if cfg!(windows) {
        "x86_64-windows"
    } else {
        "x86_64-linux"
    };

    let core_cases: Vec<(&str, &str)> = vec![
        ("examples/pipeline_validation.klik", "Pipeline result: 24"),
        ("examples/benchmark.klik", "=== KLIK Benchmark Suite ==="),
        ("examples/test_print.klik", "42"),
        ("examples/test_if.klik", "1"),
        ("examples/test_pipe.klik", "36"),
        ("examples/stress.klik", "499999500000"),
    ];

    println!("CORE TEST RESULTS\n-----------------");
    let mut core_failures: Vec<String> = Vec::new();
    let mut benchmark_native: Option<BackendRunResult> = None;
    let mut benchmark_cranelift: Option<BackendRunResult> = None;

    for (example, expected) in &core_cases {
        let source = cwd.join(example);
        let case_name = Path::new(example)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(example);

        let native = run_example_backend(&cwd, &source, "native", false);
        let cranelift = run_example_backend(&cwd, &source, cranelift_target, false);

        match (native, cranelift) {
            (Ok(native_res), Ok(cranelift_res)) => {
                let native_ok = native_res.output.contains(expected);
                let cranelift_ok = cranelift_res.output.contains(expected);
                let equal = native_res.output.trim() == cranelift_res.output.trim();

                if case_name == "benchmark" {
                    benchmark_native = Some(BackendRunResult {
                        output: native_res.output.clone(),
                        compile_ms: native_res.compile_ms,
                        exec_secs: native_res.exec_secs,
                    });
                    benchmark_cranelift = Some(BackendRunResult {
                        output: cranelift_res.output.clone(),
                        compile_ms: cranelift_res.compile_ms,
                        exec_secs: cranelift_res.exec_secs,
                    });
                }

                if native_ok && cranelift_ok && equal {
                    println!("PASS {}", case_name);
                } else {
                    println!("FAIL {} (wrong output)", case_name);
                    core_failures.push(format!(
                        "{} output mismatch\n  native:\n{}\n  cranelift:\n{}",
                        case_name,
                        native_res.output.trim(),
                        cranelift_res.output.trim()
                    ));
                }
            }
            (Err(e), Ok(_)) => {
                println!(
                    "FAIL {} (native {})",
                    case_name,
                    classify_backend_error(&format!("{:#}", e)).to_lowercase()
                );
                core_failures.push(format!("{} native error: {:#}", case_name, e));
            }
            (Ok(_), Err(e)) => {
                println!(
                    "FAIL {} (cranelift {})",
                    case_name,
                    classify_backend_error(&format!("{:#}", e)).to_lowercase()
                );
                core_failures.push(format!("{} cranelift error: {:#}", case_name, e));
            }
            (Err(e1), Err(e2)) => {
                println!("FAIL {} (both backends failed)", case_name);
                core_failures.push(format!(
                    "{} failed on both backends\n  native: {:#}\n  cranelift: {:#}",
                    case_name, e1, e2
                ));
            }
        }
    }

    println!("\nADVANCED TEST RESULTS\n---------------------");
    let advanced_cases = vec![
        "examples/advanced/file_search.klik",
        "examples/advanced/game_of_life.klik",
        "examples/advanced/json_parser.klik",
        "examples/advanced/markdown_renderer.klik",
        "examples/advanced/mini_database.klik",
        "examples/advanced/prime_calculator.klik",
        "examples/advanced/todo_app.klik",
        "examples/advanced/web_server.klik",
    ];

    let mut advanced_failures: Vec<String> = Vec::new();

    for example in &advanced_cases {
        let source = cwd.join(example);
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .context("Could not infer output name for advanced example")?;
        let case_name = stem;
        let exe = default_binary_path(&cwd, stem);
        if exe.exists() {
            let _ = std::fs::remove_file(&exe);
        }

        let status = match build_with_options(
            Some(source.as_path()),
            false,
            cranelift_target,
            Some(exe.clone()),
            false,
            "O1",
            false,
            false,
            false,
            false,
        ) {
            Ok(()) => match run_binary_capture(&exe) {
                Ok(_) => {
                    println!("PASS {}", case_name);
                    None
                }
                Err(e) => {
                    println!(
                        "FAIL {} ({})",
                        case_name,
                        classify_backend_error(&format!("{:#}", e)).to_lowercase()
                    );
                    Some(format!("{}: {:#}", case_name, e))
                }
            },
            Err(e) => {
                println!(
                    "FAIL {} ({})",
                    case_name,
                    classify_backend_error(&format!("{:#}", e)).to_lowercase()
                );
                Some(format!("{}: {:#}", case_name, e))
            }
        };

        if let Some(failure) = status {
            advanced_failures.push(failure);
        }
    }

    println!("\nBackend Comparison\n------------------");
    if let (Some(native), Some(cranelift)) = (&benchmark_native, &benchmark_cranelift) {
        println!("Rust backend compile time: {} ms", native.compile_ms);
        println!(
            "Cranelift backend compile time: {} ms",
            cranelift.compile_ms
        );
        println!("\nExecution time:\n");
        println!("benchmark.klik");
        println!("Rust backend: {:.2}s", native.exec_secs);
        println!("Cranelift backend: {:.2}s", cranelift.exec_secs);
    } else {
        println!("Benchmark timings unavailable due to earlier benchmark failures");
    }

    if !advanced_failures.is_empty() {
        println!(
            "\n{} {} advanced test(s) failed (warning only)",
            "warning:".yellow().bold(),
            advanced_failures.len()
        );
    }

    if !core_failures.is_empty() {
        bail!(
            "core backend validation failed with {} failure(s)",
            core_failures.len()
        );
    }

    println!("\n{}", "All core backend checks passed".green().bold());
    Ok(())
}

/// Check project for errors
pub fn check(_all_warnings: bool) -> Result<()> {
    let timer = Instant::now();
    let cwd = std::env::current_dir()?;
    let mut pm = klik_package_manager::PackageManager::new(cwd);
    let manifest = pm.load_manifest()?;

    println!(
        "{} {} v{}",
        "Checking".green().bold(),
        manifest.package.name,
        manifest.package.version
    );

    let sources = pm.find_sources()?;
    let mut error_count = 0;
    let warning_count = 0;

    for source_path in &sources {
        let source_code = std::fs::read_to_string(source_path)?;
        let file_name = source_path.to_string_lossy().to_string();

        let tokens = match klik_lexer::Lexer::new(&source_code, &file_name).tokenize() {
            Ok(tokens) => tokens,
            Err(e) => {
                eprintln!("{} {}: {:?}", "error".red().bold(), file_name, e);
                error_count += 1;
                continue;
            }
        };

        let ast = match klik_parser::Parser::new(tokens, &file_name).parse_program() {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!("{} {}: {:?}", "error".red().bold(), file_name, e);
                error_count += 1;
                continue;
            }
        };

        let mut analyzer = klik_semantic::SemanticAnalyzer::new();
        if let Err(e) = analyzer.analyze(&ast) {
            eprintln!("{} {}: {:?}", "error".red().bold(), file_name, e);
            error_count += 1;
            continue;
        }

        let mut type_checker = klik_type_system::TypeChecker::new();
        if let Err(e) = type_checker.check_program(&ast) {
            eprintln!("{} {}: {:?}", "error".red().bold(), file_name, e);
            error_count += 1;
        }
    }

    let elapsed = timer.elapsed();

    if error_count > 0 {
        eprintln!(
            "{}: {} error(s), {} warning(s) in {:.2}s",
            "Failed".red().bold(),
            error_count,
            warning_count,
            elapsed.as_secs_f64()
        );
        bail!("check failed with {} error(s)", error_count);
    }

    println!(
        "{} check in {:.2}s ({} file(s), {} warning(s))",
        "Finished".green().bold(),
        elapsed.as_secs_f64(),
        sources.len(),
        warning_count
    );

    Ok(())
}

/// Run tests
pub fn test(filter: Option<&str>, _show_output: bool) -> Result<()> {
    let timer = Instant::now();
    let cwd = std::env::current_dir()?;
    let mut pm = klik_package_manager::PackageManager::new(cwd);
    let manifest = pm.load_manifest()?;

    println!(
        "{} tests for {} v{}",
        "Running".green().bold(),
        manifest.package.name,
        manifest.package.version
    );

    let sources = pm.find_sources()?;
    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;

    for source_path in &sources {
        let source_code = std::fs::read_to_string(source_path)?;
        let file_name = source_path.to_string_lossy().to_string();
        let tokens = klik_lexer::Lexer::new(&source_code, &file_name)
            .tokenize()
            .map_err(|e| anyhow::anyhow!("Lexer error: {:?}", e))?;
        let ast = klik_parser::Parser::new(tokens, &file_name)
            .parse_program()
            .map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;

        for module in &ast.modules {
            for item in &module.items {
                if let klik_ast::Item::Test(test) = item {
                    if let Some(f) = filter {
                        if !test.name.contains(f) {
                            continue;
                        }
                    }
                    total += 1;

                    let test_module = klik_ast::Module {
                        name: format!("__test_{}", test.name),
                        items: vec![klik_ast::Item::Function(klik_ast::Function {
                            name: format!("__test_{}", test.name),
                            params: vec![],
                            return_type: None,
                            body: test.body.clone(),
                            generic_params: vec![],
                            is_async: false,
                            is_pub: false,
                            span: test.span.clone(),
                        })],
                        span: test.span.clone(),
                    };
                    let test_program = klik_ast::Program {
                        modules: vec![test_module],
                        span: test.span.clone(),
                    };

                    let mut analyzer = klik_semantic::SemanticAnalyzer::new();
                    match analyzer.analyze(&test_program) {
                        Ok(_) => {
                            println!("  {} {}", "pass".green(), test.name);
                            passed += 1;
                        }
                        Err(e) => {
                            println!("  {} {} - {:?}", "FAIL".red(), test.name, e);
                            failed += 1;
                        }
                    }
                }
            }
        }
    }

    let elapsed = timer.elapsed();
    println!();
    if failed > 0 {
        println!(
            "{}: {} passed, {} failed, {} total ({:.2}s)",
            "FAILED".red().bold(),
            passed,
            failed,
            total,
            elapsed.as_secs_f64()
        );
        bail!("{} test(s) failed", failed);
    } else {
        println!(
            "{}: {} passed, {} total ({:.2}s)",
            "OK".green().bold(),
            passed,
            total,
            elapsed.as_secs_f64()
        );
    }

    Ok(())
}

/// Format source files
pub fn fmt(check: bool, files: &[PathBuf]) -> Result<()> {
    let cwd = std::env::current_dir()?;

    let sources = if files.is_empty() {
        let mut pm = klik_package_manager::PackageManager::new(cwd);
        pm.load_manifest()?;
        pm.find_sources()?
    } else {
        files.to_vec()
    };

    let mut formatted_count = 0;
    let mut unchanged_count = 0;

    for source_path in &sources {
        let source_code = std::fs::read_to_string(source_path)?;
        let file_name = source_path.to_string_lossy().to_string();

        let tokens = match klik_lexer::Lexer::new(&source_code, &file_name).tokenize() {
            Ok(tokens) => tokens,
            Err(_) => continue,
        };

        let ast = match klik_parser::Parser::new(tokens, &file_name).parse_program() {
            Ok(ast) => ast,
            Err(_) => continue,
        };

        let formatted = klik_formatter::format_program(&ast);

        if formatted != source_code {
            if check {
                println!("{} {}", "Unformatted:".yellow(), source_path.display());
                formatted_count += 1;
            } else {
                std::fs::write(source_path, &formatted)?;
                println!("{} {}", "Formatted:".green(), source_path.display());
                formatted_count += 1;
            }
        } else {
            unchanged_count += 1;
        }
    }

    if check && formatted_count > 0 {
        bail!("{} file(s) need formatting", formatted_count);
    }

    println!(
        "{}: {} formatted, {} unchanged",
        "Done".green().bold(),
        formatted_count,
        unchanged_count
    );

    Ok(())
}

/// Run the linter
pub fn lint(files: &[PathBuf], _fix: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    let sources = if files.is_empty() {
        let mut pm = klik_package_manager::PackageManager::new(cwd);
        pm.load_manifest()?;
        pm.find_sources()?
    } else {
        files.to_vec()
    };

    let mut total_warnings = 0;
    let mut total_errors = 0;

    for source_path in &sources {
        let source_code = std::fs::read_to_string(source_path)?;
        let file_name = source_path.to_string_lossy().to_string();

        let tokens = match klik_lexer::Lexer::new(&source_code, &file_name).tokenize() {
            Ok(tokens) => tokens,
            Err(_) => continue,
        };

        let ast = match klik_parser::Parser::new(tokens, &file_name).parse_program() {
            Ok(ast) => ast,
            Err(_) => continue,
        };

        let mut linter = klik_linter::Linter::new();
        let diagnostics = linter.lint(&ast);

        for diag in &diagnostics {
            let severity = match diag.severity {
                klik_linter::Severity::Error => {
                    total_errors += 1;
                    "error".red().bold()
                }
                klik_linter::Severity::Warning => {
                    total_warnings += 1;
                    "warning".yellow().bold()
                }
                klik_linter::Severity::Info => "info".blue().bold(),
            };
            eprintln!(
                "{}: {} ({}:{})",
                severity,
                diag.message,
                source_path.display(),
                diag.line
            );
        }
    }

    if total_errors > 0 {
        bail!("{} error(s), {} warning(s)", total_errors, total_warnings);
    }

    println!(
        "{}: {} warning(s), {} error(s)",
        "Done".green().bold(),
        total_warnings,
        total_errors
    );

    Ok(())
}

/// Add a dependency
pub fn add_dep(name: &str, version: &str, _dev: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut pm = klik_package_manager::PackageManager::new(cwd);
    pm.load_manifest()?;
    pm.add_dependency(name, version)?;
    println!(
        "{} dependency `{}` ({})",
        "Added".green().bold(),
        name,
        version
    );
    Ok(())
}

/// Remove a dependency
pub fn remove_dep(name: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut pm = klik_package_manager::PackageManager::new(cwd);
    pm.load_manifest()?;
    pm.remove_dependency(name)?;
    println!("{} dependency `{}`", "Removed".green().bold(), name);
    Ok(())
}

/// Start the language server
pub fn start_lsp() -> Result<()> {
    println!("{} KLIK Language Server...", "Starting".green().bold());
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(klik_lsp::run_server())?;
    Ok(())
}

/// Watch for changes and rebuild
pub fn watch(_args: &[String]) -> Result<()> {
    use notify::{Event, EventKind, RecursiveMode, Watcher};
    use std::sync::mpsc;

    let cwd = std::env::current_dir()?;
    let src_dir = cwd.join("src");

    if !src_dir.exists() {
        bail!("No src/ directory found");
    }

    println!(
        "{} for changes in {} ...",
        "Watching".green().bold(),
        src_dir.display()
    );

    if let Err(e) = build(None, false, "native", None, false) {
        eprintln!("{} {:#}", "Build failed:".red().bold(), e);
    }

    let (tx, rx) = mpsc::channel();

    let mut watcher =
        notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                ) {
                    let _ = tx.send(());
                }
            }
        })?;

    watcher.watch(&src_dir, RecursiveMode::Recursive)?;

    loop {
        rx.recv()?;
        std::thread::sleep(std::time::Duration::from_millis(200));
        while rx.try_recv().is_ok() {}

        println!();
        println!("{} change detected, rebuilding...", "->".cyan().bold());

        match build(None, false, "native", None, false) {
            Ok(()) => println!("{} rebuild complete", "OK".green().bold()),
            Err(e) => eprintln!("{} {:#}", "Build failed:".red().bold(), e),
        }
    }
}

/// Clean build artifacts
pub fn clean() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let target_dir = cwd.join("target");

    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)?;
        println!("{} build artifacts", "Cleaned".green().bold());
    } else {
        println!("Nothing to clean");
    }

    Ok(())
}

/// Show project information
pub fn info() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut pm = klik_package_manager::PackageManager::new(cwd);
    let manifest = pm.load_manifest()?;

    let pkg_name = manifest.package.name.clone();
    let pkg_version = manifest.package.version.clone();
    let pkg_edition = manifest.package.edition.clone();
    let pkg_desc = manifest.package.description.clone();
    let pkg_license = manifest.package.license.clone();
    let pkg_authors = manifest.package.authors.clone();
    let deps = manifest.dependencies.clone();

    println!("{}", "Project Info".bold().underline());
    println!("  Name:    {}", pkg_name.cyan());
    println!("  Version: {}", pkg_version.cyan());
    println!("  Edition: {}", pkg_edition.cyan());

    if let Some(desc) = &pkg_desc {
        println!("  Description: {}", desc);
    }
    if let Some(license) = &pkg_license {
        println!("  License: {}", license);
    }
    if !pkg_authors.is_empty() {
        println!("  Authors: {}", pkg_authors.join(", "));
    }

    let sources = pm.find_sources()?;
    println!("  Source files: {}", sources.len());

    if !deps.is_empty() {
        println!();
        println!("{}", "Dependencies".bold().underline());
        for (name, spec) in &deps {
            match spec {
                klik_package_manager::DependencySpec::Simple(v) => {
                    println!("  {} = \"{}\"", name, v);
                }
                klik_package_manager::DependencySpec::Detailed(d) => {
                    if let Some(v) = &d.version {
                        println!("  {} = \"{}\"", name, v);
                    } else if let Some(p) = &d.path {
                        println!("  {} (path: {})", name, p);
                    } else if let Some(g) = &d.git {
                        println!("  {} (git: {})", name, g);
                    }
                }
            }
        }
    }

    Ok(())
}
