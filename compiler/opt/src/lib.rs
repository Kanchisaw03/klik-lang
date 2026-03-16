use klik_ir::{
    BinOp, BlockRef, CmpOp, Instruction, IrConst, IrFunction, IrModule, Terminator, UnOp, Value,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O1,
    O2,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PassReport {
    pub folded: usize,
    pub removed: usize,
    pub simplified_blocks: usize,
    pub simplified_branches: usize,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OptimizeReport {
    pub constant_folding: PassReport,
    pub dead_code_elimination: PassReport,
    pub block_simplification: PassReport,
    pub branch_simplification: PassReport,
}

pub fn constant_folding(module: &mut IrModule) -> PassReport {
    let mut report = PassReport::default();
    for func in &mut module.functions {
        report.folded += constant_fold_function(func);
    }
    report
}

pub fn dead_code_elimination(module: &mut IrModule) -> PassReport {
    let mut report = PassReport::default();
    for func in &mut module.functions {
        report.removed += dce_function(func);
    }
    report
}

pub fn block_simplification(module: &mut IrModule) -> PassReport {
    let mut report = PassReport::default();
    for func in &mut module.functions {
        report.simplified_blocks += simplify_blocks_function(func);
    }
    report
}

pub fn branch_simplification(module: &mut IrModule) -> PassReport {
    let mut report = PassReport::default();
    for func in &mut module.functions {
        report.simplified_branches += simplify_branches_function(func);
    }
    report
}

pub fn optimize(module: &mut IrModule, level: OptLevel) -> OptimizeReport {
    let mut report = OptimizeReport::default();
    match level {
        OptLevel::O0 => report,
        OptLevel::O1 => {
            report.constant_folding = constant_folding(module);
            report
        }
        OptLevel::O2 => {
            report.constant_folding = constant_folding(module);
            report.dead_code_elimination = dead_code_elimination(module);
            report.block_simplification = block_simplification(module);
            report.branch_simplification = branch_simplification(module);
            report
        }
    }
}

fn constant_fold_function(func: &mut IrFunction) -> usize {
    let mut changed = 0usize;
    let mut constants: HashMap<Value, IrConst> = HashMap::new();

    for block in &mut func.blocks {
        let mut rewritten = Vec::with_capacity(block.instructions.len());
        for inst in &block.instructions {
            match inst {
                Instruction::Const(v, c) => {
                    constants.insert(*v, c.clone());
                    rewritten.push(inst.clone());
                }
                Instruction::BinOp(result, op, lhs, rhs) => {
                    let folded = fold_binop(*op, constants.get(lhs), constants.get(rhs));
                    if let Some(c) = folded {
                        constants.insert(*result, c.clone());
                        rewritten.push(Instruction::Const(*result, c));
                        changed += 1;
                    } else {
                        rewritten.push(inst.clone());
                    }
                }
                Instruction::ICmp(result, op, lhs, rhs) => {
                    let folded = fold_icmp(*op, constants.get(lhs), constants.get(rhs));
                    if let Some(c) = folded {
                        constants.insert(*result, c.clone());
                        rewritten.push(Instruction::Const(*result, c));
                        changed += 1;
                    } else {
                        rewritten.push(inst.clone());
                    }
                }
                Instruction::FCmp(result, op, lhs, rhs) => {
                    let folded = fold_fcmp(*op, constants.get(lhs), constants.get(rhs));
                    if let Some(c) = folded {
                        constants.insert(*result, c.clone());
                        rewritten.push(Instruction::Const(*result, c));
                        changed += 1;
                    } else {
                        rewritten.push(inst.clone());
                    }
                }
                Instruction::UnaryOp(result, op, operand) => {
                    let folded = fold_unary(*op, constants.get(operand));
                    if let Some(c) = folded {
                        constants.insert(*result, c.clone());
                        rewritten.push(Instruction::Const(*result, c));
                        changed += 1;
                    } else {
                        rewritten.push(inst.clone());
                    }
                }
                _ => rewritten.push(inst.clone()),
            }
        }
        block.instructions = rewritten;
    }

    changed
}

fn fold_binop(op: BinOp, lhs: Option<&IrConst>, rhs: Option<&IrConst>) -> Option<IrConst> {
    match (lhs, rhs) {
        (Some(IrConst::Int(l)), Some(IrConst::Int(r))) => match op {
            BinOp::IAdd => Some(IrConst::Int(l.wrapping_add(*r))),
            BinOp::ISub => Some(IrConst::Int(l.wrapping_sub(*r))),
            BinOp::IMul => Some(IrConst::Int(l.wrapping_mul(*r))),
            BinOp::IDiv if *r != 0 => Some(IrConst::Int(l.wrapping_div(*r))),
            BinOp::IMod if *r != 0 => Some(IrConst::Int(l.wrapping_rem(*r))),
            _ => None,
        },
        _ => None,
    }
}

fn fold_icmp(op: CmpOp, lhs: Option<&IrConst>, rhs: Option<&IrConst>) -> Option<IrConst> {
    match (lhs, rhs) {
        (Some(IrConst::Int(l)), Some(IrConst::Int(r))) => Some(IrConst::Bool(match op {
            CmpOp::Eq => l == r,
            CmpOp::Ne => l != r,
            CmpOp::Lt => l < r,
            CmpOp::Le => l <= r,
            CmpOp::Gt => l > r,
            CmpOp::Ge => l >= r,
        })),
        (Some(IrConst::Bool(l)), Some(IrConst::Bool(r))) => Some(IrConst::Bool(match op {
            CmpOp::Eq => l == r,
            CmpOp::Ne => l != r,
            CmpOp::Lt => (*l as i32) < (*r as i32),
            CmpOp::Le => (*l as i32) <= (*r as i32),
            CmpOp::Gt => (*l as i32) > (*r as i32),
            CmpOp::Ge => (*l as i32) >= (*r as i32),
        })),
        _ => None,
    }
}

fn fold_fcmp(op: CmpOp, lhs: Option<&IrConst>, rhs: Option<&IrConst>) -> Option<IrConst> {
    match (lhs, rhs) {
        (Some(IrConst::Float(l)), Some(IrConst::Float(r))) => Some(IrConst::Bool(match op {
            CmpOp::Eq => l == r,
            CmpOp::Ne => l != r,
            CmpOp::Lt => l < r,
            CmpOp::Le => l <= r,
            CmpOp::Gt => l > r,
            CmpOp::Ge => l >= r,
        })),
        _ => None,
    }
}

fn fold_unary(op: UnOp, operand: Option<&IrConst>) -> Option<IrConst> {
    match (op, operand) {
        (UnOp::INeg, Some(IrConst::Int(v))) => Some(IrConst::Int(v.wrapping_neg())),
        (UnOp::FNeg, Some(IrConst::Float(v))) => Some(IrConst::Float(-v)),
        (UnOp::Not, Some(IrConst::Bool(v))) => Some(IrConst::Bool(!v)),
        (UnOp::BitNot, Some(IrConst::Int(v))) => Some(IrConst::Int(!v)),
        _ => None,
    }
}

fn dce_function(func: &mut IrFunction) -> usize {
    let mut removed = 0usize;

    loop {
        let used = compute_used_values(func);
        let before: usize = func.blocks.iter().map(|b| b.instructions.len()).sum();

        for block in &mut func.blocks {
            block.instructions.retain(|inst| {
                if has_side_effect(inst) {
                    return true;
                }
                match inst.result() {
                    Some(v) => used.contains(&v),
                    None => true,
                }
            });
        }

        let after: usize = func.blocks.iter().map(|b| b.instructions.len()).sum();
        if after == before {
            break;
        }
        removed += before.saturating_sub(after);
    }

    removed
}

fn compute_used_values(func: &IrFunction) -> HashSet<Value> {
    let mut used = HashSet::new();

    for block in &func.blocks {
        if let Some(term) = &block.terminator {
            match term {
                Terminator::Return(Some(v)) => {
                    used.insert(*v);
                }
                Terminator::CondBranch(v, _, _) => {
                    used.insert(*v);
                }
                Terminator::Switch(v, cases, _) => {
                    used.insert(*v);
                    for (_, b) in cases {
                        let _ = b;
                    }
                }
                _ => {}
            }
        }

        for inst in &block.instructions {
            match inst {
                Instruction::BinOp(_, _, a, b)
                | Instruction::ICmp(_, _, a, b)
                | Instruction::FCmp(_, _, a, b)
                | Instruction::Store(a, b)
                | Instruction::GetElementPtr(_, a, b) => {
                    used.insert(*a);
                    used.insert(*b);
                }
                Instruction::StructFieldStore(base, _, value) => {
                    used.insert(*base);
                    used.insert(*value);
                }
                Instruction::UnaryOp(_, _, a)
                | Instruction::Load(_, a)
                | Instruction::Cast(_, a, _)
                | Instruction::StructFieldLoad(_, a, _) => {
                    used.insert(*a);
                }
                Instruction::Call(_, _, args) => {
                    for a in args {
                        used.insert(*a);
                    }
                }
                Instruction::Phi(_, incoming) => {
                    for (_, v) in incoming {
                        used.insert(*v);
                    }
                }
                Instruction::Const(_, _) | Instruction::Alloca(_, _) | Instruction::Nop => {}
            }
        }
    }

    used
}

fn has_side_effect(inst: &Instruction) -> bool {
    matches!(
        inst,
        Instruction::Call(_, _, _)
            | Instruction::Store(_, _)
            | Instruction::Load(_, _)
            | Instruction::Alloca(_, _)
            | Instruction::GetElementPtr(_, _, _)
            | Instruction::StructFieldLoad(_, _, _)
            | Instruction::StructFieldStore(_, _, _)
    )
}

fn simplify_blocks_function(func: &mut IrFunction) -> usize {
    let mut changed_total = 0usize;

    loop {
        let preds = compute_predecessors(func);
        let mut candidates = Vec::new();

        for (idx, block) in func.blocks.iter().enumerate() {
            if idx == 0 {
                continue;
            }
            if !block.instructions.is_empty() {
                continue;
            }
            let succ = match block.terminator {
                Some(Terminator::Branch(BlockRef(target))) => target,
                _ => continue,
            };
            if preds[idx].len() == 1 && succ != idx {
                candidates.push((idx, succ, preds[idx][0]));
            }
        }

        if candidates.is_empty() {
            break;
        }

        for (block_idx, succ_idx, pred_idx) in &candidates {
            redirect_terminator_edges(
                &mut func.blocks[*pred_idx].terminator,
                *block_idx,
                *succ_idx,
            );
            if let Some(succ) = func.blocks.get_mut(*succ_idx) {
                for inst in &mut succ.instructions {
                    if let Instruction::Phi(_, incoming) = inst {
                        for (src, _) in incoming.iter_mut() {
                            if src.0 == *block_idx {
                                src.0 = *pred_idx;
                            }
                        }
                    }
                }
            }
        }

        let remove_set: HashSet<usize> = candidates.iter().map(|(b, _, _)| *b).collect();
        if remove_set.is_empty() {
            break;
        }

        let mut remap = HashMap::new();
        let mut new_blocks = Vec::with_capacity(func.blocks.len() - remove_set.len());
        for (old, block) in func.blocks.iter().cloned().enumerate() {
            if !remove_set.contains(&old) {
                let new_idx = new_blocks.len();
                remap.insert(old, new_idx);
                new_blocks.push(block);
            }
        }

        for block in &mut new_blocks {
            if let Some(term) = &mut block.terminator {
                remap_terminator_blocks(term, &remap);
            }
            for inst in &mut block.instructions {
                if let Instruction::Phi(_, incoming) = inst {
                    for (src, _) in incoming.iter_mut() {
                        if let Some(new_idx) = remap.get(&src.0).copied() {
                            src.0 = new_idx;
                        }
                    }
                }
            }
        }

        if let Some(new_current) = remap.get(&func.current_block_idx).copied() {
            func.current_block_idx = new_current;
        } else {
            func.current_block_idx = 0;
        }

        changed_total += remove_set.len();
        func.blocks = new_blocks;
    }

    changed_total
}

fn compute_predecessors(func: &IrFunction) -> Vec<Vec<usize>> {
    let mut preds = vec![Vec::new(); func.blocks.len()];
    for (idx, block) in func.blocks.iter().enumerate() {
        if let Some(term) = &block.terminator {
            match term {
                Terminator::Branch(BlockRef(t)) => preds[*t].push(idx),
                Terminator::CondBranch(_, BlockRef(t), BlockRef(e)) => {
                    preds[*t].push(idx);
                    preds[*e].push(idx);
                }
                Terminator::Switch(_, cases, BlockRef(default)) => {
                    for (_, BlockRef(t)) in cases {
                        preds[*t].push(idx);
                    }
                    preds[*default].push(idx);
                }
                _ => {}
            }
        }
    }
    preds
}

fn redirect_terminator_edges(term: &mut Option<Terminator>, from: usize, to: usize) {
    if let Some(t) = term {
        match t {
            Terminator::Branch(BlockRef(target)) => {
                if *target == from {
                    *target = to;
                }
            }
            Terminator::CondBranch(_, BlockRef(tgt), BlockRef(els)) => {
                if *tgt == from {
                    *tgt = to;
                }
                if *els == from {
                    *els = to;
                }
            }
            Terminator::Switch(_, cases, BlockRef(default)) => {
                for (_, BlockRef(target)) in cases {
                    if *target == from {
                        *target = to;
                    }
                }
                if *default == from {
                    *default = to;
                }
            }
            _ => {}
        }
    }
}

fn remap_terminator_blocks(term: &mut Terminator, remap: &HashMap<usize, usize>) {
    match term {
        Terminator::Branch(BlockRef(target)) => {
            if let Some(new) = remap.get(target).copied() {
                *target = new;
            }
        }
        Terminator::CondBranch(_, BlockRef(then_t), BlockRef(else_t)) => {
            if let Some(new) = remap.get(then_t).copied() {
                *then_t = new;
            }
            if let Some(new) = remap.get(else_t).copied() {
                *else_t = new;
            }
        }
        Terminator::Switch(_, cases, BlockRef(default)) => {
            for (_, BlockRef(target)) in cases {
                if let Some(new) = remap.get(target).copied() {
                    *target = new;
                }
            }
            if let Some(new) = remap.get(default).copied() {
                *default = new;
            }
        }
        _ => {}
    }
}

fn simplify_branches_function(func: &mut IrFunction) -> usize {
    let mut consts = HashMap::new();
    for block in &func.blocks {
        for inst in &block.instructions {
            if let Instruction::Const(v, c) = inst {
                consts.insert(*v, c.clone());
            }
        }
    }

    let mut changed = 0usize;
    for block in &mut func.blocks {
        if let Some(Terminator::CondBranch(cond, then_b, else_b)) = block.terminator.clone() {
            let fold = match consts.get(&cond) {
                Some(IrConst::Bool(v)) => Some(*v),
                Some(IrConst::Int(v)) => Some(*v != 0),
                _ => None,
            };
            if let Some(taken_then) = fold {
                block.terminator =
                    Some(Terminator::Branch(if taken_then { then_b } else { else_b }));
                changed += 1;
            }
        }
    }

    changed
}
