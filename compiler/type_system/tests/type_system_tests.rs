use klik_type_system::{TypeChecker, TypeError};

fn type_check(source: &str) -> Result<(), Vec<TypeError>> {
    let tokens = klik_lexer::Lexer::new(source, "<test>")
        .tokenize()
        .expect("tokenize");
    let mut parser = klik_parser::Parser::new(tokens, "<test>");
    let program = parser.parse_program().expect("parse");

    let mut checker = TypeChecker::new();
    checker.check_program(&program)
}

#[test]
fn accepts_basic_builtin_types_and_functions() {
    let src = r#"
fn identity(value: int) -> int {
    value
}

fn main() {
    let a = 1
    let b = "text"
    let c = true
    let d = identity(a)
    println(to_string(d))
    println(b)
    if c { println("ok") } else { println("ko") }
}
"#;

    type_check(src).expect("program should type-check");
}

#[test]
fn reports_mismatched_assignment_types() {
    let src = r#"
fn main() {
    let x: int = "nope"
}
"#;

    let errors = type_check(src).expect_err("expected type mismatch");
    assert!(errors
        .iter()
        .any(|e| matches!(e, TypeError::Mismatch { .. })));
}

#[test]
fn accepts_println_with_any_args() {
    let src = r#"
fn main() {
    println(1)
    println("hello", 42, true)
}
"#;

    // println is variadic and accepts any types, so no errors expected
    let result = type_check(src);
    assert!(result.is_ok(), "println should accept any argument types");
}
