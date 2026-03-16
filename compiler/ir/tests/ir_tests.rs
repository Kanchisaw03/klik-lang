use klik_ir::{BinOp, Instruction, IrBuilder, IrConst, IrType, Value};

#[test]
fn lowers_basic_ast_constructs_to_ir() {
    let src = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let x = add(1, 2)
    x
}
"#;

    let tokens = klik_lexer::Lexer::new(src, "<test>")
        .tokenize()
        .expect("tokenize");
    let mut parser = klik_parser::Parser::new(tokens, "<test>");
    let program = parser.parse_program().expect("parse");

    let mut builder = IrBuilder::new("test");
    let module = builder.build_module(&program);

    assert!(module.functions.iter().any(|f| f.name == "add"));
    assert!(module.functions.iter().any(|f| f.name == "main"));

    let add_fn = module
        .functions
        .iter()
        .find(|f| f.name == "add")
        .expect("add fn");

    assert!(add_fn.blocks.iter().any(|b| {
        b.instructions
            .iter()
            .any(|i| matches!(i, Instruction::BinOp(_, BinOp::IAdd, _, _)))
    }));

    assert!(add_fn
        .blocks
        .iter()
        .any(|b| matches!(b.terminator, Some(klik_ir::Terminator::Return(_)))));
}

#[test]
fn exposes_required_instruction_shapes() {
    let load = Instruction::Load(Value(1), Value(2));
    let store = Instruction::Store(Value(3), Value(4));
    let add = Instruction::BinOp(Value(5), BinOp::IAdd, Value(6), Value(7));
    let sub = Instruction::BinOp(Value(8), BinOp::ISub, Value(9), Value(10));
    let mul = Instruction::BinOp(Value(11), BinOp::IMul, Value(12), Value(13));
    let div = Instruction::BinOp(Value(14), BinOp::IDiv, Value(15), Value(16));
    let call = Instruction::Call(Value(17), "callee".to_string(), vec![Value(18)]);
    let ret = klik_ir::Terminator::Return(Some(Value(19)));
    let constant = Instruction::Const(Value(20), IrConst::Int(1));
    let alloca = Instruction::Alloca(Value(21), IrType::I64);

    assert!(matches!(load, Instruction::Load(_, _)));
    assert!(matches!(store, Instruction::Store(_, _)));
    assert!(matches!(add, Instruction::BinOp(_, BinOp::IAdd, _, _)));
    assert!(matches!(sub, Instruction::BinOp(_, BinOp::ISub, _, _)));
    assert!(matches!(mul, Instruction::BinOp(_, BinOp::IMul, _, _)));
    assert!(matches!(div, Instruction::BinOp(_, BinOp::IDiv, _, _)));
    assert!(matches!(call, Instruction::Call(_, _, _)));
    assert!(matches!(ret, klik_ir::Terminator::Return(_)));
    assert!(matches!(constant, Instruction::Const(_, _)));
    assert!(matches!(alloca, Instruction::Alloca(_, _)));
}
