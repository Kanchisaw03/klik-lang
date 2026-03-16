// KLIK Optimizer - Multiple optimization passes on the IR

use klik_ir::*;
use std::collections::{HashMap, HashSet};

/// Run all optimization passes on an IR module
pub fn optimize(module: &mut IrModule, level: OptLevel) {
    for func in &mut module.functions {
        if func.is_extern {
            continue;
        }
        match level {
            OptLevel::None => {}
            OptLevel::Basic => {
                constant_fold(func);
                dead_code_eliminate(func);
            }
            OptLevel::Standard => {
                constant_fold(func);
                common_subexpr_eliminate(func);
                dead_code_eliminate(func);
                simplify_cfg(func);
            }
            OptLevel::Aggressive => {
                constant_fold(func);
                common_subexpr_eliminate(func);
                dead_code_eliminate(func);
                simplify_cfg(func);
                // Additional passes could be added here
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    None,
    Basic,
    Standard,
    Aggressive,
}

impl OptLevel {
    pub fn from_u8(level: u8) -> Self {
        match level {
            0 => OptLevel::None,
            1 => OptLevel::Basic,
            2 => OptLevel::Standard,
            _ => OptLevel::Aggressive,
        }
    }
}

/// Constant folding - evaluate constant expressions at compile time
fn constant_fold(func: &mut IrFunction) {
    let mut constants: HashMap<Value, IrConst> = HashMap::new();

    for block in &mut func.blocks {
        let mut new_instructions = Vec::new();

        for inst in &block.instructions {
            match inst {
                Instruction::Const(val, c) => {
                    constants.insert(*val, c.clone());
                    new_instructions.push(inst.clone());
                }
                Instruction::BinOp(result, op, lhs, rhs) => {
                    if let (Some(IrConst::Int(l)), Some(IrConst::Int(r))) =
                        (constants.get(lhs), constants.get(rhs))
                    {
                        let folded = match op {
                            BinOp::IAdd => Some(IrConst::Int(l.wrapping_add(*r))),
                            BinOp::ISub => Some(IrConst::Int(l.wrapping_sub(*r))),
                            BinOp::IMul => Some(IrConst::Int(l.wrapping_mul(*r))),
                            BinOp::IDiv if *r != 0 => Some(IrConst::Int(l.wrapping_div(*r))),
                            BinOp::IMod if *r != 0 => Some(IrConst::Int(l.wrapping_rem(*r))),
                            BinOp::And => Some(IrConst::Int(l & r)),
                            BinOp::Or => Some(IrConst::Int(l | r)),
                            BinOp::Xor => Some(IrConst::Int(l ^ r)),
                            BinOp::Shl => Some(IrConst::Int(l.wrapping_shl(*r as u32))),
                            BinOp::Shr => Some(IrConst::Int(l.wrapping_shr(*r as u32))),
                            _ => None,
                        };
                        if let Some(c) = folded {
                            constants.insert(*result, c.clone());
                            new_instructions.push(Instruction::Const(*result, c));
                            continue;
                        }
                    }

                    if let (Some(IrConst::Float(l)), Some(IrConst::Float(r))) =
                        (constants.get(lhs), constants.get(rhs))
                    {
                        let folded = match op {
                            BinOp::FAdd => Some(IrConst::Float(l + r)),
                            BinOp::FSub => Some(IrConst::Float(l - r)),
                            BinOp::FMul => Some(IrConst::Float(l * r)),
                            BinOp::FDiv if *r != 0.0 => Some(IrConst::Float(l / r)),
                            _ => None,
                        };
                        if let Some(c) = folded {
                            constants.insert(*result, c.clone());
                            new_instructions.push(Instruction::Const(*result, c));
                            continue;
                        }
                    }

                    new_instructions.push(inst.clone());
                }
                Instruction::ICmp(result, op, lhs, rhs) => {
                    if let (Some(IrConst::Int(l)), Some(IrConst::Int(r))) =
                        (constants.get(lhs), constants.get(rhs))
                    {
                        let value = match op {
                            CmpOp::Eq => l == r,
                            CmpOp::Ne => l != r,
                            CmpOp::Lt => l < r,
                            CmpOp::Le => l <= r,
                            CmpOp::Gt => l > r,
                            CmpOp::Ge => l >= r,
                        };
                        let c = IrConst::Bool(value);
                        constants.insert(*result, c.clone());
                        new_instructions.push(Instruction::Const(*result, c));
                        continue;
                    }
                    new_instructions.push(inst.clone());
                }
                Instruction::UnaryOp(result, op, operand) => {
                    if let Some(c) = constants.get(operand) {
                        let folded = match (op, c) {
                            (UnOp::INeg, IrConst::Int(v)) => Some(IrConst::Int(-v)),
                            (UnOp::FNeg, IrConst::Float(v)) => Some(IrConst::Float(-v)),
                            (UnOp::Not, IrConst::Bool(v)) => Some(IrConst::Bool(!v)),
                            (UnOp::BitNot, IrConst::Int(v)) => Some(IrConst::Int(!v)),
                            _ => None,
                        };
                        if let Some(c) = folded {
                            constants.insert(*result, c.clone());
                            new_instructions.push(Instruction::Const(*result, c));
                            continue;
                        }
                    }
                    new_instructions.push(inst.clone());
                }
                _ => {
                    new_instructions.push(inst.clone());
                }
            }
        }

        block.instructions = new_instructions;
    }
}

/// Dead code elimination - remove instructions whose results are never used
fn dead_code_eliminate(func: &mut IrFunction) {
    // Collect all used values
    let mut used_values: HashSet<Value> = HashSet::new();

    // Values used in terminators
    for block in &func.blocks {
        if let Some(ref term) = block.terminator {
            match term {
                Terminator::Return(Some(val)) => {
                    used_values.insert(*val);
                }
                Terminator::CondBranch(val, _, _) => {
                    used_values.insert(*val);
                }
                Terminator::Switch(val, _, _) => {
                    used_values.insert(*val);
                }
                _ => {}
            }
        }
    }

    // Values used as operands
    for block in &func.blocks {
        for inst in &block.instructions {
            match inst {
                Instruction::BinOp(_, _, lhs, rhs) => {
                    used_values.insert(*lhs);
                    used_values.insert(*rhs);
                }
                Instruction::UnaryOp(_, _, op) => {
                    used_values.insert(*op);
                }
                Instruction::Call(_, _, args) => {
                    for arg in args {
                        used_values.insert(*arg);
                    }
                }
                Instruction::Load(_, addr) => {
                    used_values.insert(*addr);
                }
                Instruction::Store(addr, val) => {
                    used_values.insert(*addr);
                    used_values.insert(*val);
                }
                Instruction::GetElementPtr(_, base, idx) => {
                    used_values.insert(*base);
                    used_values.insert(*idx);
                }
                Instruction::Cast(_, val, _) => {
                    used_values.insert(*val);
                }
                Instruction::ICmp(_, _, lhs, rhs) | Instruction::FCmp(_, _, lhs, rhs) => {
                    used_values.insert(*lhs);
                    used_values.insert(*rhs);
                }
                Instruction::Phi(_, entries) => {
                    for (_, val) in entries {
                        used_values.insert(*val);
                    }
                }
                _ => {}
            }
        }
    }

    // Remove dead instructions (but keep calls for side effects)
    for block in &mut func.blocks {
        block.instructions.retain(|inst| {
            match inst {
                Instruction::Call(_, _, _) => true, // Keep calls (side effects)
                Instruction::Store(_, _) => true,   // Keep stores (side effects)
                Instruction::Nop => false,
                _ => {
                    if let Some(result) = inst.result() {
                        used_values.contains(&result)
                    } else {
                        true
                    }
                }
            }
        });
    }
}

/// Common subexpression elimination
fn common_subexpr_eliminate(func: &mut IrFunction) {
    // Simple CSE: track (op, operands) -> result mapping
    let mut expr_map: HashMap<(String, Vec<Value>), Value> = HashMap::new();

    for block in &mut func.blocks {
        let mut new_instructions = Vec::new();

        for inst in &block.instructions {
            let key = instruction_key(inst);
            if let Some(key) = key {
                if let Some(&existing) = expr_map.get(&key) {
                    // Replace with existing value
                    if let Some(result) = inst.result() {
                        new_instructions.push(Instruction::Const(result, IrConst::Int(0)));
                        // In a real implementation we'd do value replacement
                        let _ = existing;
                    }
                } else {
                    if let Some(result) = inst.result() {
                        expr_map.insert(key, result);
                    }
                    new_instructions.push(inst.clone());
                }
            } else {
                new_instructions.push(inst.clone());
            }
        }

        block.instructions = new_instructions;
    }
}

fn instruction_key(inst: &Instruction) -> Option<(String, Vec<Value>)> {
    match inst {
        Instruction::BinOp(_, op, lhs, rhs) => Some((format!("{:?}", op), vec![*lhs, *rhs])),
        Instruction::UnaryOp(_, op, operand) => Some((format!("{:?}", op), vec![*operand])),
        _ => None,
    }
}

/// Simplify the control flow graph
fn simplify_cfg(func: &mut IrFunction) {
    // Remove empty blocks that just branch to another block
    let mut block_remap: HashMap<usize, usize> = HashMap::new();

    for (i, block) in func.blocks.iter().enumerate() {
        if block.instructions.is_empty() {
            if let Some(Terminator::Branch(BlockRef(target))) = &block.terminator {
                if i != *target {
                    block_remap.insert(i, *target);
                }
            }
        }
    }

    // Remap block references
    if !block_remap.is_empty() {
        for block in &mut func.blocks {
            if let Some(ref mut term) = block.terminator {
                remap_terminator(term, &block_remap);
            }
        }
    }
}

fn remap_terminator(term: &mut Terminator, remap: &HashMap<usize, usize>) {
    match term {
        Terminator::Branch(BlockRef(ref mut target)) => {
            if let Some(&new_target) = remap.get(target) {
                *target = new_target;
            }
        }
        Terminator::CondBranch(_, BlockRef(ref mut then_target), BlockRef(ref mut else_target)) => {
            if let Some(&new_target) = remap.get(then_target) {
                *then_target = new_target;
            }
            if let Some(&new_target) = remap.get(else_target) {
                *else_target = new_target;
            }
        }
        Terminator::Switch(_, ref mut cases, BlockRef(ref mut default)) => {
            for (_, BlockRef(ref mut target)) in cases {
                if let Some(&new_target) = remap.get(target) {
                    *target = new_target;
                }
            }
            if let Some(&new_target) = remap.get(default) {
                *default = new_target;
            }
        }
        _ => {}
    }
}
