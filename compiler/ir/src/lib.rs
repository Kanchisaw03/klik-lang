// KLIK Intermediate Representation
// SSA-style IR with basic blocks and control flow graphs

use klik_ast::types::Type;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// A complete IR module
#[derive(Debug, Clone)]
pub struct IrModule {
    pub name: String,
    pub functions: Vec<IrFunction>,
    pub globals: Vec<IrGlobal>,
    pub string_literals: Vec<String>,
}

impl IrModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: Vec::new(),
            globals: Vec::new(),
            string_literals: Vec::new(),
        }
    }

    pub fn add_string_literal(&mut self, s: &str) -> usize {
        if let Some(idx) = self.string_literals.iter().position(|lit| lit == s) {
            idx
        } else {
            let idx = self.string_literals.len();
            self.string_literals.push(s.to_string());
            idx
        }
    }
}

#[derive(Debug, Clone)]
pub struct IrGlobal {
    pub name: String,
    pub ty: IrType,
    pub init: Option<IrConst>,
}

/// IR function definition
#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<(String, IrType)>,
    pub return_type: IrType,
    pub blocks: Vec<BasicBlock>,
    pub current_block_idx: usize,
    pub locals: Vec<(String, IrType)>,
    pub is_extern: bool,
}

impl IrFunction {
    pub fn new(name: impl Into<String>, return_type: IrType) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            return_type,
            blocks: vec![BasicBlock::new("entry")],
            current_block_idx: 0,
            locals: Vec::new(),
            is_extern: false,
        }
    }

    pub fn add_block(&mut self, name: &str) -> usize {
        let idx = self.blocks.len();
        self.blocks.push(BasicBlock::new(name));
        idx
    }

    pub fn current_block(&mut self) -> &mut BasicBlock {
        &mut self.blocks[self.current_block_idx]
    }

    pub fn block_mut(&mut self, idx: usize) -> &mut BasicBlock {
        &mut self.blocks[idx]
    }

    pub fn set_current_block(&mut self, idx: usize) {
        self.current_block_idx = idx;
    }

    pub fn add_local(&mut self, name: impl Into<String>, ty: IrType) -> usize {
        let idx = self.locals.len();
        self.locals.push((name.into(), ty));
        idx
    }
}

/// A basic block in the control flow graph
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: String,
    pub instructions: Vec<Instruction>,
    pub terminator: Option<Terminator>,
}

impl BasicBlock {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            instructions: Vec::new(),
            terminator: None,
        }
    }

    pub fn push(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }

    pub fn terminate(&mut self, term: Terminator) {
        self.terminator = Some(term);
    }

    pub fn is_terminated(&self) -> bool {
        self.terminator.is_some()
    }
}

/// SSA value reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub u32);

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

/// Block reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockRef(pub usize);

/// IR Instructions
#[derive(Debug, Clone)]
pub enum Instruction {
    /// result = const value
    Const(Value, IrConst),
    /// result = binop(lhs, rhs)
    BinOp(Value, BinOp, Value, Value),
    /// result = unaryop(operand)
    UnaryOp(Value, UnOp, Value),
    /// result = call func(args...)
    Call(Value, String, Vec<Value>),
    /// result = load(address)
    Load(Value, Value),
    /// store(address, value)
    Store(Value, Value),
    /// result = alloca(type)
    Alloca(Value, IrType),
    /// result = gep(base, index)
    GetElementPtr(Value, Value, Value),
    /// result = cast(value, target_type)
    Cast(Value, Value, IrType),
    /// result = load(base + offset)
    StructFieldLoad(Value, Value, usize),
    /// store(base + offset, value)
    StructFieldStore(Value, usize, Value),
    /// result = phi(block1: val1, block2: val2, ...)
    Phi(Value, Vec<(BlockRef, Value)>),
    /// result = icmp(op, lhs, rhs)
    ICmp(Value, CmpOp, Value, Value),
    /// result = fcmp(op, lhs, rhs)
    FCmp(Value, CmpOp, Value, Value),
    /// No operation
    Nop,
}

impl Instruction {
    pub fn result(&self) -> Option<Value> {
        match self {
            Instruction::Const(v, _) => Some(*v),
            Instruction::BinOp(v, _, _, _) => Some(*v),
            Instruction::UnaryOp(v, _, _) => Some(*v),
            Instruction::Call(v, _, _) => Some(*v),
            Instruction::Load(v, _) => Some(*v),
            Instruction::Alloca(v, _) => Some(*v),
            Instruction::GetElementPtr(v, _, _) => Some(*v),
            Instruction::Cast(v, _, _) => Some(*v),
            Instruction::StructFieldLoad(v, _, _) => Some(*v),
            Instruction::Phi(v, _) => Some(*v),
            Instruction::ICmp(v, _, _, _) => Some(*v),
            Instruction::FCmp(v, _, _, _) => Some(*v),
            Instruction::Store(_, _)
            | Instruction::StructFieldStore(_, _, _)
            | Instruction::Nop => None,
        }
    }
}

/// Block terminator instructions
#[derive(Debug, Clone)]
pub enum Terminator {
    Return(Option<Value>),
    Branch(BlockRef),
    CondBranch(Value, BlockRef, BlockRef),
    Switch(Value, Vec<(IrConst, BlockRef)>, BlockRef),
    Unreachable,
}

/// Binary operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    IAdd,
    ISub,
    IMul,
    IDiv,
    IMod,
    FAdd,
    FSub,
    FMul,
    FDiv,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

/// Unary operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    INeg,
    FNeg,
    Not,
    BitNot,
}

/// Comparison operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// IR constant values
#[derive(Debug, Clone, PartialEq)]
pub enum IrConst {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(usize), // index into string table
    Void,
}

/// IR type representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IrType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Ptr,
    Void,
    Struct(String),
    Array(Box<IrType>, usize),
    Function(Vec<IrType>, Box<IrType>),
}

impl IrType {
    pub fn from_ast_type(ty: &Type) -> Self {
        match ty {
            Type::Int | Type::Int64 => IrType::I64,
            Type::Int8 => IrType::I8,
            Type::Int16 => IrType::I16,
            Type::Int32 => IrType::I32,
            Type::Uint | Type::Uint64 => IrType::U64,
            Type::Uint8 => IrType::U8,
            Type::Uint16 => IrType::U16,
            Type::Uint32 => IrType::U32,
            Type::Float32 => IrType::F32,
            Type::Float64 => IrType::F64,
            Type::Bool => IrType::Bool,
            Type::Char => IrType::I32,
            Type::String => IrType::Ptr,
            Type::Void | Type::Never => IrType::Void,
            Type::Array(inner, size) => {
                IrType::Array(Box::new(IrType::from_ast_type(inner)), size.unwrap_or(0))
            }
            Type::Struct(name, _) => IrType::Struct(name.clone()),
            Type::Reference(_, _) => IrType::Ptr,
            Type::Optional(inner) => IrType::from_ast_type(inner), // simplified
            Type::Function(params, ret) => IrType::Function(
                params.iter().map(IrType::from_ast_type).collect(),
                Box::new(IrType::from_ast_type(ret)),
            ),
            _ => IrType::I64,
        }
    }

    pub fn size_bytes(&self) -> usize {
        match self {
            IrType::I8 | IrType::U8 | IrType::Bool => 1,
            IrType::I16 | IrType::U16 => 2,
            IrType::I32 | IrType::U32 | IrType::F32 => 4,
            IrType::I64 | IrType::U64 | IrType::F64 | IrType::Ptr => 8,
            IrType::Void => 0,
            IrType::Struct(_) => 8, // placeholder
            IrType::Array(inner, count) => inner.size_bytes() * count,
            IrType::Function(_, _) => 8,
        }
    }
}

#[derive(Debug, Clone)]
struct StructFieldLayout {
    offset: usize,
}

#[derive(Debug, Clone)]
struct StructLayout {
    fields: HashMap<String, StructFieldLayout>,
    size_bytes: usize,
}

#[derive(Debug, Clone, Copy)]
enum PipelineStage<'a> {
    Map(&'a klik_ast::LambdaExpr),
    Filter(&'a klik_ast::LambdaExpr),
    Sum,
}

/// IR builder - translates AST to IR
pub struct IrBuilder {
    module: IrModule,
    next_value: u32,
    var_map: HashMap<String, Value>,
    var_struct_types: HashMap<String, String>,
    array_bindings: HashMap<String, Vec<klik_ast::Expr>>,
    struct_layouts: HashMap<String, StructLayout>,
    enum_variant_tags: HashMap<String, i64>,
}

impl IrBuilder {
    pub fn new(module_name: &str) -> Self {
        Self {
            module: IrModule::new(module_name),
            next_value: 0,
            var_map: HashMap::new(),
            var_struct_types: HashMap::new(),
            array_bindings: HashMap::new(),
            struct_layouts: HashMap::new(),
            enum_variant_tags: HashMap::new(),
        }
    }

    pub fn fresh_value(&mut self) -> Value {
        let v = Value(self.next_value);
        self.next_value += 1;
        v
    }

    pub fn build_module(&mut self, program: &klik_ast::Program) -> IrModule {
        for module in &program.modules {
            self.build_ast_module(module);
        }
        self.module.clone()
    }

    fn build_ast_module(&mut self, module: &klik_ast::Module) {
        // Pass 1: collect type metadata needed by IR lowering.
        for item in &module.items {
            match item {
                klik_ast::Item::Struct(s) => self.register_struct_layout(s),
                klik_ast::Item::Enum(e) => self.register_enum_variants(e),
                _ => {}
            }
        }

        // Pass 2: lower executable items.
        for item in &module.items {
            match item {
                klik_ast::Item::Function(f) => {
                    let ir_func = self.build_function(f, None);
                    self.module.functions.push(ir_func);
                }
                klik_ast::Item::Impl(impl_block) => {
                    for method in &impl_block.methods {
                        let ir_func = self.build_function(method, Some(&impl_block.type_name));
                        self.module.functions.push(ir_func);
                    }
                }
                klik_ast::Item::Const(c) => {
                    let init = self.const_from_expr(&c.value);
                    self.module.globals.push(IrGlobal {
                        name: c.name.clone(),
                        ty: IrType::I64,
                        init,
                    });
                }
                _ => {}
            }
        }
    }

    fn build_function(
        &mut self,
        func: &klik_ast::Function,
        impl_self_type: Option<&str>,
    ) -> IrFunction {
        // IR values are function-local SSA IDs.
        self.next_value = 0;
        self.var_map.clear();
        self.var_struct_types.clear();
        self.array_bindings.clear();

        let ret_type = func
            .return_type
            .as_ref()
            .map(|t| self.type_expr_to_ir(t))
            .unwrap_or(IrType::Void);

        let mut ir_func = IrFunction::new(&func.name, ret_type);

        // Add parameters
        for param in &func.params {
            let ty = self.type_expr_to_ir(&param.type_expr);
            ir_func.params.push((param.name.clone(), ty.clone()));
            let val = self.fresh_value();
            self.var_map.insert(param.name.clone(), val);

            if let Some(struct_name) =
                self.resolve_struct_name_from_type_expr(&param.type_expr, impl_self_type)
            {
                self.var_struct_types
                    .insert(param.name.clone(), struct_name);
            }
        }

        // Build body
        let result = self.build_block(&func.body, &mut ir_func);

        // Add return if block is not terminated
        if !ir_func.current_block().is_terminated() {
            ir_func
                .current_block()
                .terminate(Terminator::Return(result));
        }

        ir_func
    }

    fn build_block(&mut self, block: &klik_ast::Block, func: &mut IrFunction) -> Option<Value> {
        let mut last_val = None;
        for stmt in &block.stmts {
            last_val = self.build_stmt(stmt, func);
        }
        last_val
    }

    fn build_stmt(&mut self, stmt: &klik_ast::Stmt, func: &mut IrFunction) -> Option<Value> {
        match stmt {
            klik_ast::Stmt::Let(s) => {
                if let Some(ref value) = s.value {
                    let val = self.build_expr(value, func);
                    self.var_map.insert(s.name.clone(), val);

                    if let Some(struct_name) = self.resolve_struct_name_from_expr(value) {
                        self.var_struct_types.insert(s.name.clone(), struct_name);
                    } else {
                        self.var_struct_types.remove(&s.name);
                    }

                    if let Some(elements) = self.extract_array_elements(value) {
                        self.array_bindings.insert(s.name.clone(), elements);
                    } else {
                        self.array_bindings.remove(&s.name);
                    }
                }
                None
            }
            klik_ast::Stmt::Expr(expr) => Some(self.build_expr(expr, func)),
            klik_ast::Stmt::Return(ret) => {
                let val = ret.value.as_ref().map(|v| self.build_expr(v, func));
                func.current_block().terminate(Terminator::Return(val));
                None
            }
            klik_ast::Stmt::While(w) => {
                let mut modified_vars = HashSet::new();
                self.collect_modified_vars_block(&w.body, &mut modified_vars);

                let pre_loop_map = self.var_map.clone();
                let mut loop_vars: Vec<(String, Value)> = modified_vars
                    .into_iter()
                    .filter_map(|name| pre_loop_map.get(&name).copied().map(|v| (name, v)))
                    .collect();
                loop_vars.sort_by(|a, b| a.0.cmp(&b.0));

                let cond_block = func.add_block("while.cond");
                let body_block = func.add_block("while.body");
                let end_block = func.add_block("while.end");

                let start_block = func.current_block_idx;
                func.set_current_block(start_block);
                func.current_block()
                    .terminate(Terminator::Branch(BlockRef(cond_block)));

                // Condition
                func.set_current_block(cond_block);
                let mut loop_phis: HashMap<String, Value> = HashMap::new();
                for (name, incoming_val) in &loop_vars {
                    let phi_val = self.fresh_value();
                    // Backedge incoming value is patched after body lowering.
                    func.current_block().push(Instruction::Phi(
                        phi_val,
                        vec![(BlockRef(start_block), *incoming_val)],
                    ));
                    self.var_map.insert(name.clone(), phi_val);
                    loop_phis.insert(name.clone(), phi_val);
                }

                let cond_val = self.build_expr(&w.condition, func);
                func.current_block().terminate(Terminator::CondBranch(
                    cond_val,
                    BlockRef(body_block),
                    BlockRef(end_block),
                ));

                // Body
                func.set_current_block(body_block);
                self.build_block(&w.body, func);

                let body_exit = func.current_block_idx;
                if !func.block_mut(body_exit).is_terminated() {
                    func.block_mut(body_exit)
                        .terminate(Terminator::Branch(BlockRef(cond_block)));
                }

                let has_backedge = matches!(
                    func.blocks[body_exit].terminator,
                    Some(Terminator::Branch(BlockRef(target))) if target == cond_block
                );

                if has_backedge {
                    let mut backedge_values: HashMap<Value, Value> = HashMap::new();
                    for (name, incoming_val) in &loop_vars {
                        if let Some(phi_val) = loop_phis.get(name).copied() {
                            let backedge_val =
                                self.var_map.get(name).copied().unwrap_or(*incoming_val);
                            backedge_values.insert(phi_val, backedge_val);
                        }
                    }

                    for inst in &mut func.block_mut(cond_block).instructions {
                        if let Instruction::Phi(phi_val, incoming) = inst {
                            if let Some(backedge_val) = backedge_values.get(phi_val).copied() {
                                incoming.push((BlockRef(body_exit), backedge_val));
                            }
                        }
                    }
                }

                // Drop body-local SSA names and publish loop-carried values at loop exit.
                self.var_map = pre_loop_map;
                for (name, incoming_val) in &loop_vars {
                    if let Some(phi_val) = loop_phis.get(name).copied() {
                        self.var_map.insert(name.clone(), phi_val);
                    } else {
                        self.var_map.insert(name.clone(), *incoming_val);
                    }
                }

                func.set_current_block(end_block);

                None
            }
            klik_ast::Stmt::For(f) => {
                let iter_val = self.build_expr(&f.iterator, func);
                self.var_map.insert(f.variable.clone(), iter_val);
                self.build_block(&f.body, func);
                None
            }
            klik_ast::Stmt::Assign(a) => {
                let val = self.build_expr(&a.value, func);
                if let klik_ast::Expr::Identifier(ident) = &a.target {
                    if let Some(op) = &a.op {
                        if let Some(&prev_val) = self.var_map.get(&ident.name) {
                            let bin_op = match op {
                                klik_ast::BinaryOp::Add => BinOp::IAdd,
                                klik_ast::BinaryOp::Sub => BinOp::ISub,
                                klik_ast::BinaryOp::Mul => BinOp::IMul,
                                klik_ast::BinaryOp::Div => BinOp::IDiv,
                                _ => BinOp::IAdd,
                            };
                            let result = self.fresh_value();
                            func.current_block()
                                .push(Instruction::BinOp(result, bin_op, prev_val, val));
                            self.var_map.insert(ident.name.clone(), result);
                        }
                    } else {
                        self.var_map.insert(ident.name.clone(), val);
                    }

                    if let Some(struct_name) = self.resolve_struct_name_from_expr(&a.value) {
                        self.var_struct_types
                            .insert(ident.name.clone(), struct_name);
                    } else {
                        self.var_struct_types.remove(&ident.name);
                    }

                    if let Some(elements) = self.extract_array_elements(&a.value) {
                        self.array_bindings.insert(ident.name.clone(), elements);
                    } else {
                        self.array_bindings.remove(&ident.name);
                    }
                }
                None
            }
            klik_ast::Stmt::Break(_) => None,
            klik_ast::Stmt::Continue(_) => None,
            klik_ast::Stmt::Item(_) => None,
        }
    }

    fn build_expr(&mut self, expr: &klik_ast::Expr, func: &mut IrFunction) -> Value {
        if let Some(v) = self.try_lower_pipeline_expr(expr, func) {
            return v;
        }

        match expr {
            klik_ast::Expr::Literal(lit) => {
                let val = self.fresh_value();
                let constant = match &lit.kind {
                    klik_ast::LiteralKind::Int(v) => IrConst::Int(*v),
                    klik_ast::LiteralKind::Float(v) => IrConst::Float(*v),
                    klik_ast::LiteralKind::Bool(v) => IrConst::Bool(*v),
                    klik_ast::LiteralKind::Char(v) => IrConst::Char(*v),
                    klik_ast::LiteralKind::String(s) => {
                        let idx = self.module.add_string_literal(s);
                        IrConst::String(idx)
                    }
                    klik_ast::LiteralKind::None => IrConst::Void,
                };
                func.current_block().push(Instruction::Const(val, constant));
                val
            }
            klik_ast::Expr::Identifier(ident) => {
                if let Some(&val) = self.var_map.get(&ident.name) {
                    val
                } else if let Some(tag) = self.enum_variant_tags.get(&ident.name).copied() {
                    let val = self.fresh_value();
                    func.current_block()
                        .push(Instruction::Const(val, IrConst::Int(tag)));
                    val
                } else {
                    let val = self.fresh_value();
                    func.current_block()
                        .push(Instruction::Const(val, IrConst::Int(0)));
                    val
                }
            }
            klik_ast::Expr::Binary(bin) => {
                if bin.op == klik_ast::BinaryOp::Pipe {
                    return self.build_pipe_expr(&bin.left, &bin.right, func);
                }

                let left = self.build_expr(&bin.left, func);
                let right = self.build_expr(&bin.right, func);
                let result = self.fresh_value();

                match bin.op {
                    klik_ast::BinaryOp::Add => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::IAdd,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::Sub => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::ISub,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::Mul => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::IMul,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::Div => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::IDiv,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::Mod => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::IMod,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::Eq => {
                        func.current_block()
                            .push(Instruction::ICmp(result, CmpOp::Eq, left, right))
                    }
                    klik_ast::BinaryOp::Neq => {
                        func.current_block()
                            .push(Instruction::ICmp(result, CmpOp::Ne, left, right))
                    }
                    klik_ast::BinaryOp::Lt => {
                        func.current_block()
                            .push(Instruction::ICmp(result, CmpOp::Lt, left, right))
                    }
                    klik_ast::BinaryOp::Gt => {
                        func.current_block()
                            .push(Instruction::ICmp(result, CmpOp::Gt, left, right))
                    }
                    klik_ast::BinaryOp::Lte => {
                        func.current_block()
                            .push(Instruction::ICmp(result, CmpOp::Le, left, right))
                    }
                    klik_ast::BinaryOp::Gte => {
                        func.current_block()
                            .push(Instruction::ICmp(result, CmpOp::Ge, left, right))
                    }
                    klik_ast::BinaryOp::And => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::And,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::Or => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::Or,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::BitAnd => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::And,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::BitOr => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::Or,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::BitXor => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::Xor,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::Shl => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::Shl,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::Shr => func.current_block().push(Instruction::BinOp(
                        result,
                        BinOp::Shr,
                        left,
                        right,
                    )),
                    klik_ast::BinaryOp::Pipe => unreachable!(),
                };

                result
            }
            klik_ast::Expr::Unary(unary) => {
                let operand = self.build_expr(&unary.operand, func);
                let result = self.fresh_value();
                match unary.op {
                    klik_ast::UnaryOp::Neg => {
                        func.current_block()
                            .push(Instruction::UnaryOp(result, UnOp::INeg, operand))
                    }
                    klik_ast::UnaryOp::Not => {
                        func.current_block()
                            .push(Instruction::UnaryOp(result, UnOp::Not, operand))
                    }
                    klik_ast::UnaryOp::BitNot => func.current_block().push(Instruction::UnaryOp(
                        result,
                        UnOp::BitNot,
                        operand,
                    )),
                    _ => func
                        .current_block()
                        .push(Instruction::Const(result, IrConst::Int(0))),
                }
                result
            }
            klik_ast::Expr::Call(call) => {
                let args: Vec<Value> = call.args.iter().map(|a| self.build_expr(a, func)).collect();
                let result = self.fresh_value();
                let name = if let klik_ast::Expr::Identifier(ident) = &*call.callee {
                    ident.name.clone()
                } else {
                    "_anon".into()
                };
                func.current_block()
                    .push(Instruction::Call(result, name, args));
                result
            }
            klik_ast::Expr::MethodCall(call) => {
                let receiver = self.build_expr(&call.receiver, func);
                let mut args = Vec::with_capacity(call.args.len() + 1);
                args.push(receiver);
                for arg in &call.args {
                    args.push(self.build_expr(arg, func));
                }

                let result = self.fresh_value();
                func.current_block()
                    .push(Instruction::Call(result, call.method.clone(), args));
                result
            }
            klik_ast::Expr::FieldAccess(field) => {
                let base = self.build_expr(&field.object, func);
                let offset = self
                    .resolve_field_offset(&field.object, &field.field)
                    .unwrap_or(0);
                let result = self.fresh_value();
                func.current_block()
                    .push(Instruction::StructFieldLoad(result, base, offset));
                result
            }
            klik_ast::Expr::If(if_expr) => {
                let cond = self.build_expr(&if_expr.condition, func);
                let start_block = func.current_block_idx;
                let then_block = func.add_block("if.then");
                let else_block = func.add_block("if.else");
                let merge_block = func.add_block("if.merge");
                let incoming_vars = self.var_map.clone();

                func.set_current_block(start_block);
                func.current_block().terminate(Terminator::CondBranch(
                    cond,
                    BlockRef(then_block),
                    BlockRef(else_block),
                ));

                // Then
                func.set_current_block(then_block);
                self.var_map = incoming_vars.clone();
                let then_val = self.build_block(&if_expr.then_block, func);
                let then_exit = func.current_block_idx;
                if !func.block_mut(then_exit).is_terminated() {
                    func.block_mut(then_exit)
                        .terminate(Terminator::Branch(BlockRef(merge_block)));
                }
                let then_reaches_merge = matches!(
                    func.blocks[then_exit].terminator,
                    Some(Terminator::Branch(BlockRef(target))) if target == merge_block
                );
                let then_vars = self.var_map.clone();

                // Else
                func.set_current_block(else_block);
                self.var_map = incoming_vars.clone();
                let mut else_val = None;
                if let Some(ref else_expr) = if_expr.else_block {
                    else_val = Some(self.build_expr(else_expr, func));
                }
                let else_exit = func.current_block_idx;
                if !func.block_mut(else_exit).is_terminated() {
                    func.block_mut(else_exit)
                        .terminate(Terminator::Branch(BlockRef(merge_block)));
                }
                let else_reaches_merge = matches!(
                    func.blocks[else_exit].terminator,
                    Some(Terminator::Branch(BlockRef(target))) if target == merge_block
                );
                let else_vars = self.var_map.clone();

                func.set_current_block(merge_block);

                // Merge branch-updated variable versions for side-effectful if expressions.
                let mut merged_vars = incoming_vars.clone();
                let mut candidate_vars: HashSet<String> = HashSet::new();
                candidate_vars.extend(then_vars.keys().cloned());
                candidate_vars.extend(else_vars.keys().cloned());

                for name in candidate_vars {
                    let incoming = incoming_vars.get(&name).copied();
                    let then_v = then_vars.get(&name).copied().or(incoming);
                    let else_v = else_vars.get(&name).copied().or(incoming);

                    let merged = match (then_reaches_merge, else_reaches_merge, then_v, else_v) {
                        (true, true, Some(tv), Some(ev)) => {
                            if tv == ev {
                                Some(tv)
                            } else {
                                let phi = self.fresh_value();
                                func.current_block().push(Instruction::Phi(
                                    phi,
                                    vec![(BlockRef(then_exit), tv), (BlockRef(else_exit), ev)],
                                ));
                                Some(phi)
                            }
                        }
                        (true, false, Some(tv), _) => Some(tv),
                        (false, true, _, Some(ev)) => Some(ev),
                        _ => incoming,
                    };

                    if let Some(v) = merged {
                        merged_vars.insert(name, v);
                    }
                }
                self.var_map = merged_vars;

                if let (Some(tv), Some(ev)) = (then_val, else_val) {
                    match (then_reaches_merge, else_reaches_merge) {
                        (true, true) => {
                            if tv == ev {
                                tv
                            } else {
                                let result = self.fresh_value();
                                func.current_block().push(Instruction::Phi(
                                    result,
                                    vec![(BlockRef(then_exit), tv), (BlockRef(else_exit), ev)],
                                ));
                                result
                            }
                        }
                        (true, false) => tv,
                        (false, true) => ev,
                        (false, false) => {
                            let v = self.fresh_value();
                            func.current_block()
                                .push(Instruction::Const(v, IrConst::Void));
                            v
                        }
                    }
                } else {
                    let v = self.fresh_value();
                    func.current_block()
                        .push(Instruction::Const(v, IrConst::Void));
                    v
                }
            }
            klik_ast::Expr::Match(match_expr) => self.build_match_expr(match_expr, func),
            klik_ast::Expr::Block(block) => self.build_block(block, func).unwrap_or_else(|| {
                let v = self.fresh_value();
                func.current_block()
                    .push(Instruction::Const(v, IrConst::Void));
                v
            }),
            klik_ast::Expr::Array(arr) => {
                let mut vals = Vec::new();
                for elem in &arr.elements {
                    vals.push(self.build_expr(elem, func));
                }
                vals.first().copied().unwrap_or_else(|| {
                    let v = self.fresh_value();
                    func.current_block()
                        .push(Instruction::Const(v, IrConst::Void));
                    v
                })
            }
            klik_ast::Expr::StructInit(si) => {
                let size_bytes = self
                    .struct_layouts
                    .get(&si.name)
                    .map(|layout| layout.size_bytes)
                    .unwrap_or(8)
                    .max(1);

                let base = self.fresh_value();
                func.current_block().push(Instruction::Alloca(
                    base,
                    IrType::Array(Box::new(IrType::I8), size_bytes),
                ));

                for (field_name, field_expr) in &si.fields {
                    let field_val = self.build_expr(field_expr, func);
                    let offset = self
                        .struct_layouts
                        .get(&si.name)
                        .and_then(|layout| layout.fields.get(field_name))
                        .map(|f| f.offset)
                        .unwrap_or(0);
                    func.current_block()
                        .push(Instruction::StructFieldStore(base, offset, field_val));
                }

                base
            }
            _ => {
                let v = self.fresh_value();
                func.current_block()
                    .push(Instruction::Const(v, IrConst::Int(0)));
                v
            }
        }
    }

    fn build_pipe_expr(
        &mut self,
        left: &klik_ast::Expr,
        right: &klik_ast::Expr,
        func: &mut IrFunction,
    ) -> Value {
        let left_val = self.build_expr(left, func);

        let (callee_name, call_args) = match right {
            klik_ast::Expr::Call(call) => {
                let mut args = Vec::with_capacity(call.args.len() + 1);
                args.push(left_val);
                for arg in &call.args {
                    args.push(self.build_expr(arg, func));
                }

                let callee_name = if let klik_ast::Expr::Identifier(ident) = call.callee.as_ref() {
                    ident.name.clone()
                } else {
                    "_anon_pipe".to_string()
                };

                (callee_name, args)
            }
            klik_ast::Expr::Identifier(ident) => (ident.name.clone(), vec![left_val]),
            _ => {
                let rhs_val = self.build_expr(right, func);
                ("_pipe_rhs".to_string(), vec![left_val, rhs_val])
            }
        };

        let result = self.fresh_value();
        func.current_block()
            .push(Instruction::Call(result, callee_name, call_args));
        result
    }

    fn append_pipeline_stage<'a>(
        stage_expr: &'a klik_ast::Expr,
        out: &mut Vec<PipelineStage<'a>>,
    ) -> bool {
        match stage_expr {
            klik_ast::Expr::Call(call) => {
                let name = if let klik_ast::Expr::Identifier(id) = call.callee.as_ref() {
                    id.name.as_str()
                } else {
                    return false;
                };

                match name {
                    "map" => {
                        if let Some(klik_ast::Expr::Lambda(lambda)) = call.args.first() {
                            out.push(PipelineStage::Map(lambda));
                            true
                        } else {
                            false
                        }
                    }
                    "filter" => {
                        if let Some(klik_ast::Expr::Lambda(lambda)) = call.args.first() {
                            out.push(PipelineStage::Filter(lambda));
                            true
                        } else {
                            false
                        }
                    }
                    "sum" if call.args.is_empty() => {
                        out.push(PipelineStage::Sum);
                        true
                    }
                    _ => false,
                }
            }
            klik_ast::Expr::Identifier(ident) if ident.name == "sum" => {
                out.push(PipelineStage::Sum);
                true
            }
            _ => false,
        }
    }

    fn collect_pipe_stages<'a>(
        left: &'a klik_ast::Expr,
        right: &'a klik_ast::Expr,
        out: &mut Vec<PipelineStage<'a>>,
    ) -> Option<&'a klik_ast::Expr> {
        let source = if let klik_ast::Expr::Binary(bin) = left {
            if bin.op == klik_ast::BinaryOp::Pipe {
                Self::collect_pipe_stages(&bin.left, &bin.right, out)?
            } else {
                left
            }
        } else {
            left
        };

        if Self::append_pipeline_stage(right, out) {
            Some(source)
        } else {
            None
        }
    }

    fn extract_canonical_pipeline<'a>(
        expr: &'a klik_ast::Expr,
    ) -> Option<(&'a klik_ast::Expr, Vec<PipelineStage<'a>>)> {
        if let klik_ast::Expr::Binary(bin) = expr {
            if bin.op == klik_ast::BinaryOp::Pipe {
                let mut stages = Vec::new();
                let source = Self::collect_pipe_stages(&bin.left, &bin.right, &mut stages)?;
                return Some((source, stages));
            }
        }

        if let klik_ast::Expr::Call(call) = expr {
            let callee = if let klik_ast::Expr::Identifier(id) = call.callee.as_ref() {
                id.name.as_str()
            } else {
                return None;
            };

            let (source, stage) = match callee {
                "map" if call.args.len() >= 2 => {
                    if let klik_ast::Expr::Lambda(lambda) = &call.args[1] {
                        (&call.args[0], PipelineStage::Map(lambda))
                    } else {
                        return None;
                    }
                }
                "filter" if call.args.len() >= 2 => {
                    if let klik_ast::Expr::Lambda(lambda) = &call.args[1] {
                        (&call.args[0], PipelineStage::Filter(lambda))
                    } else {
                        return None;
                    }
                }
                "sum" if !call.args.is_empty() => (&call.args[0], PipelineStage::Sum),
                _ => return None,
            };

            if let Some((root, mut stages)) = Self::extract_canonical_pipeline(source) {
                stages.push(stage);
                return Some((root, stages));
            }
            return Some((source, vec![stage]));
        }

        None
    }

    fn try_lower_pipeline_expr(
        &mut self,
        expr: &klik_ast::Expr,
        func: &mut IrFunction,
    ) -> Option<Value> {
        let (source, stages) = Self::extract_canonical_pipeline(expr)?;
        if stages.is_empty() || !matches!(stages.last(), Some(PipelineStage::Sum)) {
            return None;
        }

        let elements = self.extract_array_elements(source)?;

        let mut acc = self.fresh_value();
        func.current_block()
            .push(Instruction::Const(acc, IrConst::Int(0)));

        for elem in elements {
            let mut value = self.build_expr(&elem, func);
            let mut include = self.bool_const(true, func);

            for stage in &stages[..stages.len() - 1] {
                match stage {
                    PipelineStage::Map(lambda) => {
                        value = self.apply_lambda_with_arg(lambda, value, func);
                    }
                    PipelineStage::Filter(lambda) => {
                        let cond = self.apply_lambda_with_arg(lambda, value, func);
                        let combined = self.fresh_value();
                        func.current_block().push(Instruction::BinOp(
                            combined,
                            BinOp::And,
                            include,
                            cond,
                        ));
                        include = combined;
                    }
                    PipelineStage::Sum => {}
                }
            }

            let from_block = func.current_block_idx;
            let then_block = func.add_block("pipe.sum.then");
            let else_block = func.add_block("pipe.sum.else");
            let merge_block = func.add_block("pipe.sum.merge");

            func.set_current_block(from_block);
            func.current_block().terminate(Terminator::CondBranch(
                include,
                BlockRef(then_block),
                BlockRef(else_block),
            ));

            func.set_current_block(then_block);
            let then_acc = self.fresh_value();
            func.current_block()
                .push(Instruction::BinOp(then_acc, BinOp::IAdd, acc, value));
            func.current_block()
                .terminate(Terminator::Branch(BlockRef(merge_block)));

            func.set_current_block(else_block);
            func.current_block()
                .terminate(Terminator::Branch(BlockRef(merge_block)));

            func.set_current_block(merge_block);
            let merged_acc = self.fresh_value();
            func.current_block().push(Instruction::Phi(
                merged_acc,
                vec![
                    (BlockRef(then_block), then_acc),
                    (BlockRef(else_block), acc),
                ],
            ));
            acc = merged_acc;
        }

        Some(acc)
    }

    fn apply_lambda_with_arg(
        &mut self,
        lambda: &klik_ast::LambdaExpr,
        arg: Value,
        func: &mut IrFunction,
    ) -> Value {
        if lambda.params.is_empty() {
            return self.build_expr(&lambda.body, func);
        }

        let param_name = lambda.params[0].name.clone();
        let prev = self.var_map.insert(param_name.clone(), arg);
        let out = self.build_expr(&lambda.body, func);

        if let Some(prev_val) = prev {
            self.var_map.insert(param_name, prev_val);
        } else {
            self.var_map.remove(&param_name);
        }

        out
    }

    fn bool_const(&mut self, value: bool, func: &mut IrFunction) -> Value {
        let v = self.fresh_value();
        func.current_block()
            .push(Instruction::Const(v, IrConst::Bool(value)));
        v
    }

    fn build_pattern_condition(
        &mut self,
        subject: Value,
        pattern: &klik_ast::Pattern,
        func: &mut IrFunction,
    ) -> Value {
        match pattern {
            klik_ast::Pattern::Wildcard(_) | klik_ast::Pattern::Identifier(_, _) => {
                self.bool_const(true, func)
            }
            klik_ast::Pattern::Literal(lit) => {
                let rhs = self.fresh_value();
                let c = match &lit.kind {
                    klik_ast::LiteralKind::Int(v) => IrConst::Int(*v),
                    klik_ast::LiteralKind::Bool(v) => IrConst::Bool(*v),
                    klik_ast::LiteralKind::Char(v) => IrConst::Char(*v),
                    _ => IrConst::Int(0),
                };
                func.current_block().push(Instruction::Const(rhs, c));
                let cmp = self.fresh_value();
                func.current_block()
                    .push(Instruction::ICmp(cmp, CmpOp::Eq, subject, rhs));
                cmp
            }
            klik_ast::Pattern::Enum { name, variant, .. } => {
                let rhs = self.fresh_value();
                let tag = self
                    .enum_variant_tags
                    .get(&format!("{}::{}", name, variant))
                    .copied()
                    .unwrap_or(0);
                func.current_block()
                    .push(Instruction::Const(rhs, IrConst::Int(tag)));
                let cmp = self.fresh_value();
                func.current_block()
                    .push(Instruction::ICmp(cmp, CmpOp::Eq, subject, rhs));
                cmp
            }
            klik_ast::Pattern::Or(patterns, _) => {
                let mut acc = self.bool_const(false, func);
                for p in patterns {
                    let cond = self.build_pattern_condition(subject, p, func);
                    let next = self.fresh_value();
                    func.current_block()
                        .push(Instruction::BinOp(next, BinOp::Or, acc, cond));
                    acc = next;
                }
                acc
            }
            _ => self.bool_const(false, func),
        }
    }

    fn build_match_expr(&mut self, m: &klik_ast::MatchExpr, func: &mut IrFunction) -> Value {
        if m.arms.is_empty() {
            let v = self.fresh_value();
            func.current_block()
                .push(Instruction::Const(v, IrConst::Void));
            return v;
        }

        let subject = self.build_expr(&m.subject, func);
        let incoming_vars = self.var_map.clone();
        let merge_block = func.add_block("match.merge");

        let mut check_block = func.current_block_idx;
        let mut arm_results: Vec<(usize, Value)> = Vec::new();
        let mut arm_var_maps: Vec<HashMap<String, Value>> = Vec::new();

        for (idx, arm) in m.arms.iter().enumerate() {
            let arm_block = func.add_block("match.arm");
            let next_block = func.add_block("match.next");

            func.set_current_block(check_block);
            self.var_map = incoming_vars.clone();

            let mut cond = self.build_pattern_condition(subject, &arm.pattern, func);
            if let Some(guard) = &arm.guard {
                let guard_val = self.build_expr(guard, func);
                let and_val = self.fresh_value();
                func.current_block()
                    .push(Instruction::BinOp(and_val, BinOp::And, cond, guard_val));
                cond = and_val;
            }

            func.current_block().terminate(Terminator::CondBranch(
                cond,
                BlockRef(arm_block),
                BlockRef(next_block),
            ));

            func.set_current_block(arm_block);
            self.var_map = incoming_vars.clone();
            let arm_value = self.build_expr(&arm.body, func);
            let arm_exit = func.current_block_idx;
            if !func.block_mut(arm_exit).is_terminated() {
                func.block_mut(arm_exit)
                    .terminate(Terminator::Branch(BlockRef(merge_block)));
            }
            let reaches_merge = matches!(
                func.blocks[arm_exit].terminator,
                Some(Terminator::Branch(BlockRef(target))) if target == merge_block
            );
            if reaches_merge {
                arm_results.push((arm_exit, arm_value));
                arm_var_maps.push(self.var_map.clone());
            }

            check_block = next_block;

            if idx == m.arms.len() - 1 {
                break;
            }
        }

        // Default path when no arm condition matches.
        func.set_current_block(check_block);
        self.var_map = incoming_vars.clone();
        let default_value = self.fresh_value();
        func.current_block()
            .push(Instruction::Const(default_value, IrConst::Int(0)));
        let default_exit = func.current_block_idx;
        if !func.block_mut(default_exit).is_terminated() {
            func.block_mut(default_exit)
                .terminate(Terminator::Branch(BlockRef(merge_block)));
        }
        arm_results.push((default_exit, default_value));
        arm_var_maps.push(self.var_map.clone());

        func.set_current_block(merge_block);

        let mut merged_vars = incoming_vars.clone();
        let mut candidate_vars: HashSet<String> = HashSet::new();
        for vars in &arm_var_maps {
            candidate_vars.extend(vars.keys().cloned());
        }

        for name in candidate_vars {
            let mut incoming: Vec<(usize, Value)> = Vec::new();
            for (idx, vars) in arm_var_maps.iter().enumerate() {
                let (_, block_val) = arm_results[idx];
                let v = vars
                    .get(&name)
                    .copied()
                    .or_else(|| incoming_vars.get(&name).copied())
                    .unwrap_or(block_val);
                incoming.push((arm_results[idx].0, v));
            }

            let first = incoming.first().copied();
            if let Some((_, first_val)) = first {
                if incoming.iter().all(|(_, v)| *v == first_val) {
                    merged_vars.insert(name, first_val);
                } else {
                    let phi = self.fresh_value();
                    let phi_incoming: Vec<(BlockRef, Value)> = incoming
                        .into_iter()
                        .map(|(b, v)| (BlockRef(b), v))
                        .collect();
                    func.current_block()
                        .push(Instruction::Phi(phi, phi_incoming));
                    merged_vars.insert(name, phi);
                }
            }
        }
        self.var_map = merged_vars;

        if arm_results.len() == 1 {
            return arm_results[0].1;
        }

        let first_val = arm_results[0].1;
        if arm_results.iter().all(|(_, v)| *v == first_val) {
            return first_val;
        }

        let result = self.fresh_value();
        let phi_incoming: Vec<(BlockRef, Value)> = arm_results
            .into_iter()
            .map(|(b, v)| (BlockRef(b), v))
            .collect();
        func.current_block()
            .push(Instruction::Phi(result, phi_incoming));
        result
    }

    fn register_struct_layout(&mut self, def: &klik_ast::StructDef) {
        let mut fields = HashMap::new();
        let mut offset = 0usize;
        for field in &def.fields {
            let ty = self.type_expr_to_ir(&field.type_expr);
            let size = ty.size_bytes().max(1);
            fields.insert(field.name.clone(), StructFieldLayout { offset });
            offset += size;
        }

        self.struct_layouts.insert(
            def.name.clone(),
            StructLayout {
                fields,
                size_bytes: offset.max(1),
            },
        );
    }

    fn register_enum_variants(&mut self, def: &klik_ast::EnumDef) {
        for (idx, variant) in def.variants.iter().enumerate() {
            self.enum_variant_tags
                .insert(format!("{}::{}", def.name, variant.name), idx as i64);
        }
    }

    fn resolve_struct_name_from_type_expr(
        &self,
        ty: &klik_ast::TypeExpr,
        impl_self_type: Option<&str>,
    ) -> Option<String> {
        match ty {
            klik_ast::TypeExpr::Named { name, .. } if name == "Self" => {
                impl_self_type.map(|s| s.to_string())
            }
            klik_ast::TypeExpr::Named { name, .. } => {
                self.struct_layouts.contains_key(name).then(|| name.clone())
            }
            _ => None,
        }
    }

    fn resolve_struct_name_from_expr(&self, expr: &klik_ast::Expr) -> Option<String> {
        match expr {
            klik_ast::Expr::StructInit(si) => Some(si.name.clone()),
            klik_ast::Expr::Identifier(ident) => self.var_struct_types.get(&ident.name).cloned(),
            klik_ast::Expr::Cast(c) => self.resolve_struct_name_from_expr(&c.expr),
            _ => None,
        }
    }

    fn resolve_field_offset(&self, object: &klik_ast::Expr, field: &str) -> Option<usize> {
        let struct_name = self.resolve_struct_name_from_expr(object)?;
        self.struct_layouts
            .get(&struct_name)
            .and_then(|layout| layout.fields.get(field))
            .map(|f| f.offset)
    }

    fn extract_array_elements(&self, expr: &klik_ast::Expr) -> Option<Vec<klik_ast::Expr>> {
        match expr {
            klik_ast::Expr::Array(arr) => Some(arr.elements.clone()),
            klik_ast::Expr::Identifier(ident) => self.array_bindings.get(&ident.name).cloned(),
            _ => None,
        }
    }

    fn collect_modified_vars_block(&self, block: &klik_ast::Block, out: &mut HashSet<String>) {
        for stmt in &block.stmts {
            self.collect_modified_vars_stmt(stmt, out);
        }
    }

    fn collect_modified_vars_stmt(&self, stmt: &klik_ast::Stmt, out: &mut HashSet<String>) {
        match stmt {
            klik_ast::Stmt::Let(let_stmt) => {
                out.insert(let_stmt.name.clone());
                if let Some(value) = &let_stmt.value {
                    self.collect_modified_vars_expr(value, out);
                }
            }
            klik_ast::Stmt::Expr(expr) => self.collect_modified_vars_expr(expr, out),
            klik_ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.collect_modified_vars_expr(value, out);
                }
            }
            klik_ast::Stmt::While(while_stmt) => {
                self.collect_modified_vars_expr(&while_stmt.condition, out);
                self.collect_modified_vars_block(&while_stmt.body, out);
            }
            klik_ast::Stmt::For(for_stmt) => {
                out.insert(for_stmt.variable.clone());
                self.collect_modified_vars_expr(&for_stmt.iterator, out);
                self.collect_modified_vars_block(&for_stmt.body, out);
            }
            klik_ast::Stmt::Assign(assign_stmt) => {
                if let klik_ast::Expr::Identifier(ident) = &assign_stmt.target {
                    out.insert(ident.name.clone());
                }
                self.collect_modified_vars_expr(&assign_stmt.value, out);
            }
            klik_ast::Stmt::Break(_) | klik_ast::Stmt::Continue(_) | klik_ast::Stmt::Item(_) => {}
        }
    }

    fn collect_modified_vars_expr(&self, expr: &klik_ast::Expr, out: &mut HashSet<String>) {
        match expr {
            klik_ast::Expr::Binary(bin) => {
                self.collect_modified_vars_expr(&bin.left, out);
                self.collect_modified_vars_expr(&bin.right, out);
            }
            klik_ast::Expr::Unary(unary) => self.collect_modified_vars_expr(&unary.operand, out),
            klik_ast::Expr::Call(call) => {
                self.collect_modified_vars_expr(&call.callee, out);
                for arg in &call.args {
                    self.collect_modified_vars_expr(arg, out);
                }
            }
            klik_ast::Expr::MethodCall(call) => {
                self.collect_modified_vars_expr(&call.receiver, out);
                for arg in &call.args {
                    self.collect_modified_vars_expr(arg, out);
                }
            }
            klik_ast::Expr::FieldAccess(field) => {
                self.collect_modified_vars_expr(&field.object, out);
            }
            klik_ast::Expr::If(if_expr) => {
                self.collect_modified_vars_expr(&if_expr.condition, out);
                self.collect_modified_vars_block(&if_expr.then_block, out);
                if let Some(else_expr) = &if_expr.else_block {
                    self.collect_modified_vars_expr(else_expr, out);
                }
            }
            klik_ast::Expr::Match(m) => {
                self.collect_modified_vars_expr(&m.subject, out);
                for arm in &m.arms {
                    if let Some(guard) = &arm.guard {
                        self.collect_modified_vars_expr(guard, out);
                    }
                    self.collect_modified_vars_expr(&arm.body, out);
                }
            }
            klik_ast::Expr::Block(block) => self.collect_modified_vars_block(block, out),
            klik_ast::Expr::Array(arr) => {
                for elem in &arr.elements {
                    self.collect_modified_vars_expr(elem, out);
                }
            }
            klik_ast::Expr::StructInit(si) => {
                for (_, expr) in &si.fields {
                    self.collect_modified_vars_expr(expr, out);
                }
            }
            klik_ast::Expr::Lambda(lambda) => self.collect_modified_vars_expr(&lambda.body, out),
            _ => {}
        }
    }

    fn type_expr_to_ir(&self, type_expr: &klik_ast::TypeExpr) -> IrType {
        match type_expr {
            klik_ast::TypeExpr::Named { name, .. } => match name.as_str() {
                "int" | "i64" => IrType::I64,
                "i32" => IrType::I32,
                "i16" => IrType::I16,
                "i8" => IrType::I8,
                "uint" | "u64" => IrType::U64,
                "u32" => IrType::U32,
                "u16" => IrType::U16,
                "u8" => IrType::U8,
                "f32" => IrType::F32,
                "f64" => IrType::F64,
                "bool" => IrType::Bool,
                "string" => IrType::Ptr,
                "void" => IrType::Void,
                other => IrType::Struct(other.to_string()),
            },
            klik_ast::TypeExpr::Array { element, .. } => {
                IrType::Array(Box::new(self.type_expr_to_ir(element)), 0)
            }
            klik_ast::TypeExpr::Reference { .. } => IrType::Ptr,
            _ => IrType::I64,
        }
    }

    fn const_from_expr(&mut self, expr: &klik_ast::Expr) -> Option<IrConst> {
        match expr {
            klik_ast::Expr::Literal(lit) => match &lit.kind {
                klik_ast::LiteralKind::Int(v) => Some(IrConst::Int(*v)),
                klik_ast::LiteralKind::Float(v) => Some(IrConst::Float(*v)),
                klik_ast::LiteralKind::Bool(v) => Some(IrConst::Bool(*v)),
                _ => None,
            },
            _ => None,
        }
    }
}
