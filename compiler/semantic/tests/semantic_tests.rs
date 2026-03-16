use klik_semantic::{SemanticAnalyzer, SemanticError};

fn analyze(source: &str) -> Result<(), Vec<SemanticError>> {
    let tokens = klik_lexer::Lexer::new(source, "<test>")
        .tokenize()
        .expect("tokenize");
    let mut parser = klik_parser::Parser::new(tokens, "<test>");
    let program = parser.parse_program().expect("parse");

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program)
}

#[test]
fn reports_undefined_variable() {
    let src = r#"
fn main() {
    x
}
"#;

    let errors = analyze(src).expect_err("expected semantic error");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemanticError::UndefinedSymbol { name, .. } if name == "x")));
}

#[test]
fn reports_duplicate_variable_in_same_scope() {
    let src = r#"
fn main() {
    let x = 1
    let x = 2
}
"#;

    let errors = analyze(src).expect_err("expected semantic error");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemanticError::DuplicateDefinition { name, .. } if name == "x")));
}

#[test]
fn allows_shadowing_in_nested_scope() {
    let src = r#"
fn main() {
    let x = 1
    if true {
        let x = 2
        println("inner")
    }
    x
}
"#;

    analyze(src).expect("shadowing in nested block should be valid");
}

#[test]
fn resolves_function_declarations_before_use() {
    let src = r#"
fn main() {
    foo()
}

fn foo() {
    println("ok")
}
"#;

    analyze(src).expect("function should resolve from top-level declarations pass");
}
