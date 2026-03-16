use klik_ir::{
    BasicBlock, BinOp, Instruction, IrConst, IrFunction, IrModule, IrType, Terminator, Value,
};
use klik_optimizer::{optimize, OptLevel};

#[test]
fn constant_folding_rewrites_integer_binops() {
    let mut module = IrModule::new("test");
    let mut func = IrFunction::new("main", IrType::I64);

    let mut block = BasicBlock::new("entry");
    block.push(Instruction::Const(Value(0), IrConst::Int(1)));
    block.push(Instruction::Const(Value(1), IrConst::Int(2)));
    block.push(Instruction::BinOp(
        Value(2),
        BinOp::IAdd,
        Value(0),
        Value(1),
    ));
    block.terminate(Terminator::Return(Some(Value(2))));

    func.blocks = vec![block];
    module.functions.push(func);

    optimize(&mut module, OptLevel::Basic);

    let instructions = &module.functions[0].blocks[0].instructions;
    assert!(instructions
        .iter()
        .any(|inst| matches!(inst, Instruction::Const(Value(2), IrConst::Int(3)))));
}

#[test]
fn dead_code_eliminates_unused_values() {
    let mut module = IrModule::new("test");
    let mut func = IrFunction::new("main", IrType::I64);

    let mut block = BasicBlock::new("entry");
    block.push(Instruction::Const(Value(0), IrConst::Int(10)));
    block.push(Instruction::Const(Value(1), IrConst::Int(20))); // dead
    block.terminate(Terminator::Return(Some(Value(0))));

    func.blocks = vec![block];
    module.functions.push(func);

    optimize(&mut module, OptLevel::Basic);

    let instructions = &module.functions[0].blocks[0].instructions;
    assert!(instructions
        .iter()
        .any(|inst| matches!(inst, Instruction::Const(Value(0), IrConst::Int(10)))));
    assert!(!instructions
        .iter()
        .any(|inst| matches!(inst, Instruction::Const(Value(1), IrConst::Int(20)))));
}
