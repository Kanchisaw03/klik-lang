use klik_ast::{Expr, Item, LiteralKind, Stmt};

#[test]
fn parses_empty_main_function() {
    let src = "fn main() {}";
    let program = klik_parser::parse(src, "<test>").expect("parse");

    assert_eq!(program.modules.len(), 1);
    assert_eq!(program.modules[0].items.len(), 1);
    match &program.modules[0].items[0] {
        Item::Function(f) => {
            assert_eq!(f.name, "main");
            assert!(f.body.stmts.is_empty());
        }
        other => panic!("expected function, got {other:?}"),
    }
}

#[test]
fn parses_let_and_arithmetic_expression() {
    let src = r#"
fn main() {
    let x = 5
    x + 2
}
"#;

    let program = klik_parser::parse(src, "<test>").expect("parse");
    let Item::Function(main_fn) = &program.modules[0].items[0] else {
        panic!("expected main function");
    };

    assert_eq!(main_fn.body.stmts.len(), 2);

    match &main_fn.body.stmts[0] {
        Stmt::Let(let_stmt) => {
            assert_eq!(let_stmt.name, "x");
            let Some(Expr::Literal(lit)) = let_stmt.value.as_ref() else {
                panic!("expected literal initializer");
            };
            assert!(matches!(lit.kind, LiteralKind::Int(5)));
        }
        other => panic!("expected let statement, got {other:?}"),
    }

    match &main_fn.body.stmts[1] {
        Stmt::Expr(Expr::Binary(_)) => {}
        other => panic!("expected binary expression, got {other:?}"),
    }
}
