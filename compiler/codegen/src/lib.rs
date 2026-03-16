// KLIK Code Generator - Cranelift backend for native + WASM output

use anyhow::{bail, Result};
use cranelift::prelude::*;
use cranelift_codegen::ir::function::Function as CraneliftFunction;
use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use klik_ir::{
    BinOp, BlockRef, CmpOp, Instruction, IrConst, IrFunction, IrModule, IrType, Terminator, UnOp,
    Value as IrValue,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct PhiNode {
    result: IrValue,
    ty: cranelift_codegen::ir::Type,
    incomings: Vec<(usize, IrValue)>,
}

#[derive(Clone, Debug)]
struct RuntimePrintData {
    print_s_id: FuncId,
    print_i64_id: FuncId,
    space_data: DataId,
    newline_data: DataId,
}

/// Target platform for code generation
#[derive(Debug, Clone)]
pub enum Target {
    Native,
    Wasm,
    // Cross-compilation targets
    X86_64Linux,
    X86_64MacOS,
    X86_64Windows,
    Aarch64Linux,
    Aarch64MacOS,
}

impl Target {
    pub fn triple(&self) -> cranelift_codegen::isa::OwnedTargetIsa {
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").unwrap();
        match self {
            Target::Native => {
                flag_builder.set("is_pic", "true").unwrap();
                cranelift_native::builder_with_options(true)
                    .unwrap()
                    .finish(settings::Flags::new(flag_builder))
                    .unwrap()
            }
            _ => {
                let triple_str = match self {
                    Target::Wasm => "wasm32-unknown-unknown",
                    Target::X86_64Linux => "x86_64-unknown-linux-gnu",
                    Target::X86_64MacOS => "x86_64-apple-darwin",
                    Target::X86_64Windows => "x86_64-pc-windows-msvc",
                    Target::Aarch64Linux => "aarch64-unknown-linux-gnu",
                    Target::Aarch64MacOS => "aarch64-apple-darwin",
                    Target::Native => unreachable!(),
                };
                let triple: target_lexicon::Triple = triple_str.parse().unwrap();
                cranelift_codegen::isa::lookup(triple)
                    .unwrap()
                    .finish(settings::Flags::new(flag_builder))
                    .unwrap()
            }
        }
    }
}

/// Code generator using Cranelift
pub struct CodeGenerator {
    target: Target,
}

impl CodeGenerator {
    pub fn new(target: Target) -> Self {
        Self { target }
    }

    /// Generate an object file from IR
    pub fn generate(&self, ir_module: &IrModule) -> Result<Vec<u8>> {
        let isa = self.target.triple();

        let obj_builder = ObjectBuilder::new(
            isa,
            ir_module.name.clone(),
            cranelift_module::default_libcall_names(),
        )
        .unwrap();

        let mut module = ObjectModule::new(obj_builder);
        let mut ctx = module.make_context();
        let mut data_description = DataDescription::new();
        let ptr_type = module.target_config().pointer_type();

        // Declare all string literals as data sections
        let mut string_data_ids = Vec::new();
        for (i, s) in ir_module.string_literals.iter().enumerate() {
            let name = format!(".str.{}", i);
            let data_id = module.declare_data(&name, Linkage::Local, false, false)?;
            let mut bytes = s.as_bytes().to_vec();
            bytes.push(0);
            data_description.define(bytes.into_boxed_slice());
            module.define_data(data_id, &data_description)?;
            data_description.clear();
            string_data_ids.push(data_id);
        }

        let mut declare_runtime_cstr =
            |name: &str, text: &str, module: &mut ObjectModule| -> Result<DataId> {
                let data_id = module.declare_data(name, Linkage::Local, false, false)?;
                let mut bytes = text.as_bytes().to_vec();
                bytes.push(0);
                data_description.define(bytes.into_boxed_slice());
                module.define_data(data_id, &data_description)?;
                data_description.clear();
                Ok(data_id)
            };

        // First pass: declare all functions
        let mut func_ids: HashMap<String, FuncId> = HashMap::new();

        // Declare external runtime print wrappers.
        let print_s_sig = {
            let mut sig = module.make_signature();
            sig.params.push(AbiParam::new(ptr_type));
            sig.returns.push(AbiParam::new(cl_types::I32));
            sig
        };
        let print_i64_sig = {
            let mut sig = module.make_signature();
            sig.params.push(AbiParam::new(ptr_type));
            sig.returns.push(AbiParam::new(cl_types::I32));
            sig
        };

        let print_s_id = module.declare_function("klik_print_s", Linkage::Import, &print_s_sig)?;
        let print_i64_id =
            module.declare_function("klik_print_i64", Linkage::Import, &print_i64_sig)?;

        let runtime_print_data = RuntimePrintData {
            print_s_id,
            print_i64_id,
            space_data: declare_runtime_cstr(".print.space", " ", &mut module)?,
            newline_data: declare_runtime_cstr(".print.newline", "\n", &mut module)?,
        };

        for ir_func in &ir_module.functions {
            let sig = self.build_signature(ir_func, &module);
            let linkage = if ir_func.name == "main" {
                Linkage::Export
            } else {
                Linkage::Local
            };
            let func_id = module.declare_function(&ir_func.name, linkage, &sig)?;
            func_ids.insert(ir_func.name.clone(), func_id);
        }

        // Second pass: define all functions
        for ir_func in &ir_module.functions {
            if ir_func.is_extern {
                continue;
            }

            let func_id = func_ids[&ir_func.name];
            let sig = self.build_signature(ir_func, &module);

            ctx.func = CraneliftFunction::with_name_signature(
                cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32()),
                sig,
            );

            self.translate_function(
                ir_func,
                &mut ctx,
                &mut module,
                &func_ids,
                &string_data_ids,
                &runtime_print_data,
            )?;

            if let Err(err) = module.define_function(func_id, &mut ctx) {
                let verifier_message = err.to_string();
                let suggestion = self.verifier_suggestion(&verifier_message);
                bail!(
                    "Cranelift verifier failed for function `{}`\nsignature: {}\nfirst verifier line: {}\nsuggestion: {}\nir blocks:\n{}\nfull verifier output:\n{}",
                    ir_func.name,
                    self.describe_signature(ir_func),
                    verifier_message.lines().next().unwrap_or("<unknown>"),
                    suggestion,
                    self.describe_ir_blocks(ir_func),
                    verifier_message
                );
            }
            ctx.clear();
        }

        let product = module.finish();
        let bytes = product.emit()?;
        Ok(bytes)
    }

    fn build_signature(
        &self,
        ir_func: &IrFunction,
        module: &ObjectModule,
    ) -> cranelift_codegen::ir::Signature {
        let mut sig = module.make_signature();
        let ptr_type = module.target_config().pointer_type();

        for (_, ty) in &ir_func.params {
            sig.params
                .push(AbiParam::new(self.ir_type_to_cl(ty, ptr_type)));
        }

        let mut ret = self.ir_type_to_cl(&ir_func.return_type, ptr_type);
        if ir_func.name == "main" && ret == cl_types::INVALID {
            ret = cl_types::I32;
        }
        if ret != cl_types::INVALID {
            sig.returns.push(AbiParam::new(ret));
        }

        sig
    }

    fn ir_type_to_cl(
        &self,
        ty: &IrType,
        ptr_type: cranelift_codegen::ir::Type,
    ) -> cranelift_codegen::ir::Type {
        match ty {
            IrType::I8 | IrType::U8 | IrType::Bool => cl_types::I8,
            IrType::I16 | IrType::U16 => cl_types::I16,
            IrType::I32 | IrType::U32 => cl_types::I32,
            IrType::I64 | IrType::U64 => cl_types::I64,
            IrType::F32 => cl_types::F32,
            IrType::F64 => cl_types::F64,
            IrType::Ptr => ptr_type,
            IrType::Void => cl_types::INVALID,
            IrType::Struct(_) => ptr_type,
            IrType::Array(_, _) => ptr_type,
            IrType::Function(_, _) => ptr_type,
        }
    }

    fn translate_function(
        &self,
        ir_func: &IrFunction,
        ctx: &mut Context,
        module: &mut ObjectModule,
        func_ids: &HashMap<String, FuncId>,
        string_data_ids: &[cranelift_module::DataId],
        runtime_print_data: &RuntimePrintData,
    ) -> Result<()> {
        let ptr_type = module.target_config().pointer_type();
        let mut ret_ty = self.ir_type_to_cl(&ir_func.return_type, ptr_type);
        if ir_func.name == "main" && ret_ty == cl_types::INVALID {
            ret_ty = cl_types::I32;
        }
        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);

        // Create blocks
        let mut block_map: HashMap<usize, cranelift::prelude::Block> = HashMap::new();
        for (i, _) in ir_func.blocks.iter().enumerate() {
            let block = builder.create_block();
            block_map.insert(i, block);
        }

        // Add parameters to entry block
        let entry_block = block_map[&0];
        builder.append_block_params_for_function_params(entry_block);

        // Precompute value kinds and phi-node metadata so we can set block params up-front.
        let mut value_types = self.infer_value_types(ir_func, ptr_type);
        let pointer_values = self.infer_pointer_values(ir_func);
        let mut phi_nodes: HashMap<usize, Vec<PhiNode>> = HashMap::new();
        for (block_idx, block) in ir_func.blocks.iter().enumerate() {
            for inst in &block.instructions {
                if let Instruction::Phi(result, incomings) = inst {
                    let ty = incomings
                        .iter()
                        .find_map(|(_, v)| value_types.get(v).copied())
                        .unwrap_or(cl_types::I64);
                    value_types.insert(*result, ty);
                    phi_nodes.entry(block_idx).or_default().push(PhiNode {
                        result: *result,
                        ty,
                        incomings: incomings
                            .iter()
                            .map(|(BlockRef(idx), v)| (*idx, *v))
                            .collect(),
                    });
                }
            }
        }

        for (block_idx, phis) in &phi_nodes {
            let cl_block = block_map[block_idx];
            for phi in phis {
                builder.append_block_param(cl_block, phi.ty);
            }
        }

        builder.switch_to_block(entry_block);

        // Map IR values to Cranelift values
        let mut value_map: HashMap<IrValue, cranelift::prelude::Value> = HashMap::new();
        let mut _stack_slots: HashMap<IrValue, cranelift_codegen::ir::StackSlot> = HashMap::new();

        // Map function parameters
        let params = builder.block_params(entry_block).to_vec();
        for (i, (name, _)) in ir_func.params.iter().enumerate() {
            if i < params.len() {
                let param_val = IrValue(i as u32);
                value_map.insert(param_val, params[i]);
                let _ = name;
            }
        }

        if let Some(entry_phis) = phi_nodes.get(&0) {
            let base = ir_func.params.len();
            for (i, phi) in entry_phis.iter().enumerate() {
                if let Some(param) = params.get(base + i) {
                    value_map.insert(phi.result, *param);
                }
            }
        }

        // Translate each block
        for (block_idx, ir_block) in ir_func.blocks.iter().enumerate() {
            let cl_block = block_map[&block_idx];
            if block_idx > 0 {
                builder.switch_to_block(cl_block);
            }

            if let Some(phis) = phi_nodes.get(&block_idx) {
                let params = builder.block_params(cl_block).to_vec();
                let base = if block_idx == 0 { ir_func.params.len() } else { 0 };
                for (i, phi) in phis.iter().enumerate() {
                    if let Some(param) = params.get(base + i) {
                        value_map.insert(phi.result, *param);
                    }
                }
            }

            // Translate instructions
            for inst in &ir_block.instructions {
                match inst {
                    Instruction::Phi(_, _) => {}
                    Instruction::Const(result, constant) => {
                        let val = match constant {
                            IrConst::Int(v) => builder.ins().iconst(cl_types::I64, *v),
                            IrConst::Float(v) => builder.ins().f64const(*v),
                            IrConst::Bool(v) => builder.ins().iconst(cl_types::I8, *v as i64),
                            IrConst::Char(v) => builder.ins().iconst(cl_types::I32, *v as i64),
                            IrConst::String(idx) => {
                                if let Some(data_id) = string_data_ids.get(*idx) {
                                    let local = module.declare_data_in_func(*data_id, builder.func);
                                    builder.ins().symbol_value(ptr_type, local)
                                } else {
                                    builder.ins().iconst(ptr_type, 0)
                                }
                            }
                            IrConst::Void => builder.ins().iconst(cl_types::I64, 0),
                        };
                        value_map.insert(*result, val);
                    }
                    Instruction::BinOp(result, op, lhs, rhs) => {
                        let mut l = value_map
                            .get(lhs)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::I64, ptr_type));
                        let mut r = value_map
                            .get(rhs)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::I64, ptr_type));

                        let expected = match op {
                            BinOp::FAdd | BinOp::FSub | BinOp::FMul | BinOp::FDiv => cl_types::F64,
                            _ => cl_types::I64,
                        };
                        l = self.cast_value(&mut builder, l, expected, ptr_type);
                        r = self.cast_value(&mut builder, r, expected, ptr_type);

                        let val = match op {
                            BinOp::IAdd => builder.ins().iadd(l, r),
                            BinOp::ISub => builder.ins().isub(l, r),
                            BinOp::IMul => builder.ins().imul(l, r),
                            BinOp::IDiv => builder.ins().sdiv(l, r),
                            BinOp::IMod => builder.ins().srem(l, r),
                            BinOp::FAdd => builder.ins().fadd(l, r),
                            BinOp::FSub => builder.ins().fsub(l, r),
                            BinOp::FMul => builder.ins().fmul(l, r),
                            BinOp::FDiv => builder.ins().fdiv(l, r),
                            BinOp::And => builder.ins().band(l, r),
                            BinOp::Or => builder.ins().bor(l, r),
                            BinOp::Xor => builder.ins().bxor(l, r),
                            BinOp::Shl => builder.ins().ishl(l, r),
                            BinOp::Shr => builder.ins().sshr(l, r),
                        };
                        value_map.insert(*result, val);
                    }
                    Instruction::UnaryOp(result, op, operand) => {
                        let o = value_map
                            .get(operand)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::I64, ptr_type));
                        let val = match op {
                            UnOp::INeg => {
                                let casted = self.cast_value(&mut builder, o, cl_types::I64, ptr_type);
                                builder.ins().ineg(casted)
                            }
                            UnOp::FNeg => {
                                let casted = self.cast_value(&mut builder, o, cl_types::F64, ptr_type);
                                builder.ins().fneg(casted)
                            }
                            UnOp::Not => {
                                let bool_b1 = self.normalize_cond_to_b1(&mut builder, o, ptr_type);
                                let one = builder.ins().iconst(cl_types::I8, 1);
                                let zero = builder.ins().iconst(cl_types::I8, 0);
                                let as_i8 = builder.ins().select(bool_b1, one, zero);
                                let one = builder.ins().iconst(cl_types::I8, 1);
                                builder.ins().bxor(as_i8, one)
                            }
                            UnOp::BitNot => {
                                let int = self.cast_value(&mut builder, o, cl_types::I64, ptr_type);
                                builder.ins().bnot(int)
                            }
                        };
                        value_map.insert(*result, val);
                    }
                    Instruction::Call(result, name, args) => {
                        let arg_vals: Vec<cranelift::prelude::Value> = args
                            .iter()
                            .map(|a| {
                                value_map
                                    .get(a)
                                    .copied()
                                    .unwrap_or_else(|| builder.ins().iconst(cl_types::I64, 0))
                            })
                            .collect();
                        let arg_is_string: Vec<bool> = args
                            .iter()
                            .map(|a| pointer_values.get(a).copied().unwrap_or(false))
                            .collect();

                        if name == "print" || name == "println" {
                            let out = self.emit_print_sequence(
                                &mut builder,
                                module,
                                runtime_print_data,
                                &arg_vals,
                                &arg_is_string,
                                name == "println",
                                ptr_type,
                            );
                            value_map.insert(*result, out);
                            continue;
                        }

                        // print/println are currently lowered to libc puts (single argument).
                        // Keep only the first value to satisfy the callee signature.
                        if let Some(&fid) = func_ids.get(name) {
                            let func_ref = module.declare_func_in_func(fid, builder.func);
                            let call = builder.ins().call(func_ref, &arg_vals);
                            let results = builder.inst_results(call);
                            if !results.is_empty() {
                                value_map.insert(*result, results[0]);
                            } else {
                                let expected = value_types.get(result).copied().unwrap_or(cl_types::I64);
                                let zero = self.zero_value(&mut builder, expected, ptr_type);
                                value_map.insert(*result, zero);
                            }
                        } else {
                            let expected = value_types.get(result).copied().unwrap_or(cl_types::I64);
                            let zero = self.zero_value(&mut builder, expected, ptr_type);
                            value_map.insert(*result, zero);
                        }
                    }
                    Instruction::ICmp(result, op, lhs, rhs) => {
                        let l = value_map
                            .get(lhs)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::I64, ptr_type));
                        let r = value_map
                            .get(rhs)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::I64, ptr_type));
                        let l = self.cast_value(&mut builder, l, cl_types::I64, ptr_type);
                        let r = self.cast_value(&mut builder, r, cl_types::I64, ptr_type);
                        let cc = match op {
                            CmpOp::Eq => IntCC::Equal,
                            CmpOp::Ne => IntCC::NotEqual,
                            CmpOp::Lt => IntCC::SignedLessThan,
                            CmpOp::Le => IntCC::SignedLessThanOrEqual,
                            CmpOp::Gt => IntCC::SignedGreaterThan,
                            CmpOp::Ge => IntCC::SignedGreaterThanOrEqual,
                        };
                        let cmp = builder.ins().icmp(cc, l, r);
                        let one = builder.ins().iconst(cl_types::I8, 1);
                        let zero = builder.ins().iconst(cl_types::I8, 0);
                        let val = builder.ins().select(cmp, one, zero);
                        value_map.insert(*result, val);
                    }
                    Instruction::FCmp(result, op, lhs, rhs) => {
                        let l = value_map
                            .get(lhs)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::F64, ptr_type));
                        let r = value_map
                            .get(rhs)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::F64, ptr_type));
                        let l = self.cast_value(&mut builder, l, cl_types::F64, ptr_type);
                        let r = self.cast_value(&mut builder, r, cl_types::F64, ptr_type);
                        let cc = match op {
                            CmpOp::Eq => FloatCC::Equal,
                            CmpOp::Ne => FloatCC::NotEqual,
                            CmpOp::Lt => FloatCC::LessThan,
                            CmpOp::Le => FloatCC::LessThanOrEqual,
                            CmpOp::Gt => FloatCC::GreaterThan,
                            CmpOp::Ge => FloatCC::GreaterThanOrEqual,
                        };
                        let cmp = builder.ins().fcmp(cc, l, r);
                        let one = builder.ins().iconst(cl_types::I8, 1);
                        let zero = builder.ins().iconst(cl_types::I8, 0);
                        let val = builder.ins().select(cmp, one, zero);
                        value_map.insert(*result, val);
                    }
                    Instruction::Cast(result, val, target_ty) => {
                        let v = value_map
                            .get(val)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::I64, ptr_type));
                        let target = self.ir_type_to_cl(target_ty, ptr_type);
                        let cast_val = self.cast_value(&mut builder, v, target, ptr_type);
                        value_map.insert(*result, cast_val);
                    }
                    Instruction::Alloca(result, ty) => {
                        let size = ty.size_bytes().max(1) as u32;
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                size,
                                0,
                            ),
                        );
                        let ptr = builder.ins().stack_addr(ptr_type, slot, 0);
                        _stack_slots.insert(*result, slot);
                        value_map.insert(*result, ptr);
                    }
                    Instruction::Load(result, address) => {
                        let addr = value_map
                            .get(address)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, ptr_type, ptr_type));
                        let addr = self.cast_value(&mut builder, addr, ptr_type, ptr_type);
                        let val = builder
                            .ins()
                            .load(cl_types::I64, MemFlags::new(), addr, 0);
                        value_map.insert(*result, val);
                    }
                    Instruction::Store(address, value) => {
                        let addr = value_map
                            .get(address)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, ptr_type, ptr_type));
                        let addr = self.cast_value(&mut builder, addr, ptr_type, ptr_type);
                        let val = value_map
                            .get(value)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::I64, ptr_type));
                        let val = self.cast_value(&mut builder, val, cl_types::I64, ptr_type);
                        builder.ins().store(MemFlags::new(), val, addr, 0);
                    }
                    Instruction::GetElementPtr(result, base, index) => {
                        let base_ptr = value_map
                            .get(base)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, ptr_type, ptr_type));
                        let base_ptr = self.cast_value(&mut builder, base_ptr, ptr_type, ptr_type);
                        let idx = value_map
                            .get(index)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::I64, ptr_type));
                        let idx = self.cast_value(&mut builder, idx, cl_types::I64, ptr_type);
                        let scale = builder.ins().iconst(cl_types::I64, 8);
                        let byte_off = builder.ins().imul(idx, scale);
                        let byte_off = self.cast_value(&mut builder, byte_off, ptr_type, ptr_type);
                        let addr = builder.ins().iadd(base_ptr, byte_off);
                        value_map.insert(*result, addr);
                    }
                    Instruction::StructFieldLoad(result, base, offset) => {
                        let base_ptr = value_map
                            .get(base)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, ptr_type, ptr_type));
                        let base_ptr = self.cast_value(&mut builder, base_ptr, ptr_type, ptr_type);
                        let off = builder.ins().iconst(ptr_type, *offset as i64);
                        let addr = builder.ins().iadd(base_ptr, off);
                        let loaded = builder
                            .ins()
                            .load(cl_types::I64, MemFlags::new(), addr, 0);
                        value_map.insert(*result, loaded);
                    }
                    Instruction::StructFieldStore(base, offset, value) => {
                        let base_ptr = value_map
                            .get(base)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, ptr_type, ptr_type));
                        let base_ptr = self.cast_value(&mut builder, base_ptr, ptr_type, ptr_type);
                        let off = builder.ins().iconst(ptr_type, *offset as i64);
                        let addr = builder.ins().iadd(base_ptr, off);
                        let v = value_map
                            .get(value)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::I64, ptr_type));
                        let v = self.cast_value(&mut builder, v, cl_types::I64, ptr_type);
                        builder.ins().store(MemFlags::new(), v, addr, 0);
                    }
                    _ => {}
                }
            }

            // Translate terminator
            if let Some(ref term) = ir_block.terminator {
                match term {
                    Terminator::Return(val) => {
                        if ret_ty == cl_types::INVALID {
                            builder.ins().return_(&[]);
                        } else if let Some(v) = val {
                            let ret_val = value_map.get(v).copied().unwrap_or_else(|| {
                                self.zero_value(&mut builder, ret_ty, ptr_type)
                            });
                            let ret_val = self.cast_value(&mut builder, ret_val, ret_ty, ptr_type);
                            builder.ins().return_(&[ret_val]);
                        } else {
                            let zero = self.zero_value(&mut builder, ret_ty, ptr_type);
                            builder.ins().return_(&[zero]);
                        }
                    }
                    Terminator::Branch(BlockRef(target)) => {
                        let target_block = block_map[target];
                        let phi_args = self.phi_args_for_edge(
                            *target,
                            block_idx,
                            &phi_nodes,
                            &value_map,
                            &mut builder,
                            ptr_type,
                        );
                        builder.ins().jump(target_block, &phi_args);
                    }
                    Terminator::CondBranch(cond, BlockRef(then_t), BlockRef(else_t)) => {
                        let cond_val_raw = value_map
                            .get(cond)
                            .copied()
                            .unwrap_or_else(|| self.zero_value(&mut builder, cl_types::I8, ptr_type));
                        let cond_val = self.normalize_cond_to_b1(&mut builder, cond_val_raw, ptr_type);
                        let then_block = block_map[then_t];
                        let else_block = block_map[else_t];
                        let then_args = self.phi_args_for_edge(
                            *then_t,
                            block_idx,
                            &phi_nodes,
                            &value_map,
                            &mut builder,
                            ptr_type,
                        );
                        let else_args = self.phi_args_for_edge(
                            *else_t,
                            block_idx,
                            &phi_nodes,
                            &value_map,
                            &mut builder,
                            ptr_type,
                        );
                        builder
                            .ins()
                            .brif(cond_val, then_block, &then_args, else_block, &else_args);
                    }
                    Terminator::Unreachable => {
                        builder.ins().trap(TrapCode::unwrap_user(1));
                    }
                    Terminator::Switch(_, _, _) => {
                        if ret_ty == cl_types::INVALID {
                            builder.ins().return_(&[]);
                        } else {
                            let zero = self.zero_value(&mut builder, ret_ty, ptr_type);
                            builder.ins().return_(&[zero]);
                        }
                    }
                }
            } else {
                // Unterminated block: add return
                if ret_ty == cl_types::INVALID {
                    builder.ins().return_(&[]);
                } else {
                    let zero = self.zero_value(&mut builder, ret_ty, ptr_type);
                    builder.ins().return_(&[zero]);
                }
            }
        }

        // Seal all blocks after CFG edges are fully emitted.
        // This avoids late-predecessor assertions for back-edges and reordered blocks.
        builder.seal_all_blocks();
        builder.finalize();
        Ok(())
    }

    fn infer_value_types(
        &self,
        ir_func: &IrFunction,
        ptr_type: cranelift_codegen::ir::Type,
    ) -> HashMap<IrValue, cranelift_codegen::ir::Type> {
        let mut value_types = HashMap::new();

        for (idx, (_, ty)) in ir_func.params.iter().enumerate() {
            value_types.insert(IrValue(idx as u32), self.ir_type_to_cl(ty, ptr_type));
        }

        for block in &ir_func.blocks {
            for inst in &block.instructions {
                match inst {
                    Instruction::Const(v, c) => {
                        let ty = match c {
                            IrConst::Int(_) => cl_types::I64,
                            IrConst::Float(_) => cl_types::F64,
                            IrConst::Bool(_) => cl_types::I8,
                            IrConst::Char(_) => cl_types::I32,
                            IrConst::String(_) => ptr_type,
                            IrConst::Void => cl_types::I64,
                        };
                        value_types.insert(*v, ty);
                    }
                    Instruction::BinOp(v, op, _, _) => {
                        let ty = match op {
                            BinOp::FAdd | BinOp::FSub | BinOp::FMul | BinOp::FDiv => cl_types::F64,
                            _ => cl_types::I64,
                        };
                        value_types.insert(*v, ty);
                    }
                    Instruction::UnaryOp(v, op, _) => {
                        let ty = match op {
                            UnOp::FNeg => cl_types::F64,
                            UnOp::Not => cl_types::I8,
                            _ => cl_types::I64,
                        };
                        value_types.insert(*v, ty);
                    }
                    Instruction::Call(v, _, _) => {
                        value_types.insert(*v, cl_types::I64);
                    }
                    Instruction::Load(v, _)
                    | Instruction::Alloca(v, _)
                    | Instruction::GetElementPtr(v, _, _)
                    | Instruction::StructFieldLoad(v, _, _)
                    | Instruction::Cast(v, _, _)
                    | Instruction::Phi(v, _)
                    | Instruction::ICmp(v, _, _, _)
                    | Instruction::FCmp(v, _, _, _) => {
                        value_types.entry(*v).or_insert(cl_types::I64);
                    }
                    Instruction::Store(_, _)
                    | Instruction::StructFieldStore(_, _, _)
                    | Instruction::Nop => {}
                }

                if let Instruction::Cast(v, _, target_ty) = inst {
                    value_types.insert(*v, self.ir_type_to_cl(target_ty, ptr_type));
                }

                if let Instruction::ICmp(v, _, _, _) | Instruction::FCmp(v, _, _, _) = inst {
                    value_types.insert(*v, cl_types::I8);
                }
            }
        }

        value_types
    }

    fn infer_pointer_values(&self, ir_func: &IrFunction) -> HashMap<IrValue, bool> {
        let mut pointer_values = HashMap::new();

        for (idx, (_, ty)) in ir_func.params.iter().enumerate() {
            let is_ptr = matches!(
                ty,
                IrType::Ptr | IrType::Struct(_) | IrType::Array(_, _) | IrType::Function(_, _)
            );
            pointer_values.insert(IrValue(idx as u32), is_ptr);
        }

        for block in &ir_func.blocks {
            for inst in &block.instructions {
                match inst {
                    Instruction::Const(v, c) => {
                        pointer_values.insert(*v, matches!(c, IrConst::String(_)));
                    }
                    Instruction::Alloca(v, _) | Instruction::GetElementPtr(v, _, _) => {
                        pointer_values.insert(*v, true);
                    }
                    Instruction::Cast(v, _, target_ty) => {
                        let is_ptr = matches!(
                            target_ty,
                            IrType::Ptr
                                | IrType::Struct(_)
                                | IrType::Array(_, _)
                                | IrType::Function(_, _)
                        );
                        pointer_values.insert(*v, is_ptr);
                    }
                    Instruction::StructFieldLoad(v, _, _)
                    | Instruction::Load(v, _)
                    | Instruction::Call(v, _, _)
                    | Instruction::BinOp(v, _, _, _)
                    | Instruction::UnaryOp(v, _, _)
                    | Instruction::ICmp(v, _, _, _)
                    | Instruction::FCmp(v, _, _, _) => {
                        pointer_values.entry(*v).or_insert(false);
                    }
                    Instruction::Phi(v, incomings) => {
                        let any_ptr = incomings
                            .iter()
                            .any(|(_, in_v)| pointer_values.get(in_v).copied().unwrap_or(false));
                        pointer_values.entry(*v).or_insert(any_ptr);
                    }
                    Instruction::Store(_, _)
                    | Instruction::StructFieldStore(_, _, _)
                    | Instruction::Nop => {}
                }
            }
        }

        pointer_values
    }

    fn zero_value(
        &self,
        builder: &mut FunctionBuilder,
        ty: cranelift_codegen::ir::Type,
        ptr_type: cranelift_codegen::ir::Type,
    ) -> cranelift::prelude::Value {
        if ty == cl_types::INVALID {
            return builder.ins().iconst(cl_types::I64, 0);
        }
        if ty.is_float() {
            if ty == cl_types::F32 {
                return builder.ins().f32const(0.0);
            }
            return builder.ins().f64const(0.0);
        }
        if ty == ptr_type {
            return builder.ins().iconst(ptr_type, 0);
        }
        builder.ins().iconst(ty, 0)
    }

    fn cast_value(
        &self,
        builder: &mut FunctionBuilder,
        value: cranelift::prelude::Value,
        target: cranelift_codegen::ir::Type,
        ptr_type: cranelift_codegen::ir::Type,
    ) -> cranelift::prelude::Value {
        if target == cl_types::INVALID {
            return value;
        }

        let src = builder.func.dfg.value_type(value);
        if src == target {
            return value;
        }

        if src.is_int() && target.is_int() {
            if src.bits() < target.bits() {
                return builder.ins().uextend(target, value);
            }
            if src.bits() > target.bits() {
                return builder.ins().ireduce(target, value);
            }
            return value;
        }

        if src.is_int() && target.is_float() {
            return builder.ins().fcvt_from_sint(target, value);
        }

        if src.is_float() && target.is_int() {
            return builder.ins().fcvt_to_sint(target, value);
        }

        if src.is_float() && target.is_float() {
            if src == cl_types::F32 && target == cl_types::F64 {
                return builder.ins().fpromote(cl_types::F64, value);
            }
            if src == cl_types::F64 && target == cl_types::F32 {
                return builder.ins().fdemote(cl_types::F32, value);
            }
            return value;
        }

        if target == ptr_type && src.is_int() {
            if src.bits() < ptr_type.bits() {
                return builder.ins().uextend(ptr_type, value);
            }
            if src.bits() > ptr_type.bits() {
                return builder.ins().ireduce(ptr_type, value);
            }
            return value;
        }

        value
    }

    fn normalize_cond_to_b1(
        &self,
        builder: &mut FunctionBuilder,
        value: cranelift::prelude::Value,
        ptr_type: cranelift_codegen::ir::Type,
    ) -> cranelift::prelude::Value {
        let ty = builder.func.dfg.value_type(value);
        if ty.is_int() || ty == ptr_type {
            let casted = self.cast_value(builder, value, cl_types::I64, ptr_type);
            return builder.ins().icmp_imm(IntCC::NotEqual, casted, 0);
        }
        if ty.is_float() {
            let casted = self.cast_value(builder, value, cl_types::F64, ptr_type);
            let zero = builder.ins().f64const(0.0);
            return builder.ins().fcmp(FloatCC::NotEqual, casted, zero);
        }

        let zero = builder.ins().iconst(cl_types::I64, 0);
        builder.ins().icmp_imm(IntCC::NotEqual, zero, 0)
    }

    fn phi_args_for_edge(
        &self,
        target_block_idx: usize,
        from_block_idx: usize,
        phi_nodes: &HashMap<usize, Vec<PhiNode>>,
        value_map: &HashMap<IrValue, cranelift::prelude::Value>,
        builder: &mut FunctionBuilder,
        ptr_type: cranelift_codegen::ir::Type,
    ) -> Vec<cranelift::prelude::Value> {
        let mut args = Vec::new();
        if let Some(nodes) = phi_nodes.get(&target_block_idx) {
            for phi in nodes {
                let incoming = phi
                    .incomings
                    .iter()
                    .find(|(src, _)| *src == from_block_idx)
                    .and_then(|(_, v)| value_map.get(v).copied())
                    .unwrap_or_else(|| self.zero_value(builder, phi.ty, ptr_type));
                let casted = self.cast_value(builder, incoming, phi.ty, ptr_type);
                args.push(casted);
            }
        }
        args
    }

    fn data_symbol_ptr(
        &self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        data_id: DataId,
        ptr_type: cranelift_codegen::ir::Type,
    ) -> cranelift::prelude::Value {
        let local = module.declare_data_in_func(data_id, builder.func);
        builder.ins().symbol_value(ptr_type, local)
    }

    fn emit_print_s_once(
        &self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        runtime_print_data: &RuntimePrintData,
        payload_ptr: cranelift::prelude::Value,
        ptr_type: cranelift_codegen::ir::Type,
    ) -> cranelift::prelude::Value {
        let arg = self.cast_value(builder, payload_ptr, ptr_type, ptr_type);
        let print_ref = module.declare_func_in_func(runtime_print_data.print_s_id, builder.func);
        let call = builder.ins().call(print_ref, &[arg]);
        let res = builder.inst_results(call);
        if let Some(first) = res.first() {
            *first
        } else {
            self.zero_value(builder, cl_types::I32, ptr_type)
        }
    }

    fn emit_print_i64_once(
        &self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        runtime_print_data: &RuntimePrintData,
        payload: cranelift::prelude::Value,
        ptr_type: cranelift_codegen::ir::Type,
    ) -> cranelift::prelude::Value {
        let arg = self.cast_value(builder, payload, cl_types::I64, ptr_type);
        let arg = self.cast_value(builder, arg, ptr_type, ptr_type);
        let print_ref = module.declare_func_in_func(runtime_print_data.print_i64_id, builder.func);
        let call = builder.ins().call(print_ref, &[arg]);
        let res = builder.inst_results(call);
        if let Some(first) = res.first() {
            *first
        } else {
            self.zero_value(builder, cl_types::I32, ptr_type)
        }
    }

    fn emit_print_sequence(
        &self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        runtime_print_data: &RuntimePrintData,
        args: &[cranelift::prelude::Value],
        arg_is_string: &[bool],
        newline: bool,
        ptr_type: cranelift_codegen::ir::Type,
    ) -> cranelift::prelude::Value {
        let mut last = self.zero_value(builder, cl_types::I32, ptr_type);
        let space_ptr = self.data_symbol_ptr(builder, module, runtime_print_data.space_data, ptr_type);
        let newline_ptr =
            self.data_symbol_ptr(builder, module, runtime_print_data.newline_data, ptr_type);

        if args.is_empty() {
            if newline {
                last = self.emit_print_s_once(builder, module, runtime_print_data, newline_ptr, ptr_type);
            }
            return last;
        }

        for (i, arg) in args.iter().enumerate() {
            let is_last = i + 1 == args.len();
            let is_string = arg_is_string.get(i).copied().unwrap_or(false);

            if is_string {
                last = self.emit_print_s_once(builder, module, runtime_print_data, *arg, ptr_type);
            } else {
                last = self.emit_print_i64_once(builder, module, runtime_print_data, *arg, ptr_type);
            }

            if !is_last {
                last = self.emit_print_s_once(builder, module, runtime_print_data, space_ptr, ptr_type);
            }
        }

        if newline {
            last = self.emit_print_s_once(builder, module, runtime_print_data, newline_ptr, ptr_type);
        }

        last
    }

    fn describe_signature(&self, ir_func: &IrFunction) -> String {
        let params = ir_func
            .params
            .iter()
            .map(|(name, ty)| format!("{}:{:?}", name, ty))
            .collect::<Vec<_>>()
            .join(", ");
        format!("fn {}({}) -> {:?}", ir_func.name, params, ir_func.return_type)
    }

    fn describe_ir_blocks(&self, ir_func: &IrFunction) -> String {
        let mut out = String::new();
        for block in &ir_func.blocks {
            out.push_str(&format!("  block {}\n", block.label));
            for inst in &block.instructions {
                out.push_str(&format!("    {:?}\n", inst));
            }
            out.push_str(&format!("    terminator: {:?}\n", block.terminator));
        }
        out
    }

    fn verifier_suggestion(&self, message: &str) -> &'static str {
        let lower = message.to_ascii_lowercase();
        if lower.contains("arguments of return must match function signature") {
            "normalize return terminators to function return type (void should emit return without values)."
        } else if lower.contains("non-dominating") {
            "ensure values crossing CFG joins are passed via phi/block parameters instead of direct use."
        } else if lower.contains("has type") || lower.contains("mismatched") {
            "coerce integer/boolean widths at op boundaries so both operands and returns use consistent types."
        } else {
            "inspect the verifier line and emitted block listing above to locate the offending instruction."
        }
    }
}
