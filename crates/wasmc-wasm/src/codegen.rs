use std::collections::HashMap;
use wasmc_diagnostics::DiagnosticSink;
use wasmc_lexer::SymbolInterner;
use wasmc_mir::*;
use wasmc_stackifier::*;
use crate::types::*;

pub struct CodeGenerator<'a> {
    _interner: &'a SymbolInterner,
    func_indices: &'a HashMap<wasmc_lexer::SymbolId, u32>,
    func_type_indices: &'a HashMap<wasmc_lexer::SymbolId, u32>,
    stack_pointer_global: u32,
    data_end_offset: u32,
    emit_stack_check: bool,
}

#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub name: wasmc_lexer::SymbolId,
    pub type_index: u32,
    pub locals: Vec<(u32, WasmValType)>, // (count, type)
    pub body_opcodes: Vec<WasmOpcode>,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(
        interner: &'a SymbolInterner,
        func_indices: &'a HashMap<wasmc_lexer::SymbolId, u32>,
        func_type_indices: &'a HashMap<wasmc_lexer::SymbolId, u32>,
        stack_pointer_global: u32,
        data_end_offset: u32,
        emit_stack_check: bool,
    ) -> Self {
        Self {
            _interner: interner,
            func_indices,
            func_type_indices,
            stack_pointer_global,
            data_end_offset,
            emit_stack_check,
        }
    }

    pub fn compile_function(
        &self,
        func: &StructuredFunction,
        _sink: &mut DiagnosticSink,
    ) -> CompiledFunction {
        let type_index = self.func_type_indices.get(&func.name).copied().unwrap_or(0);

        // Assign Wasm local indices to SSA virtual registers
        let mut vreg_to_local: HashMap<VReg, u32> = HashMap::new();
        let mut local_idx = 0u32;

        // Function parameters already have local indices 0..params.len()
        for (vr, _) in &func.params {
            vreg_to_local.insert(*vr, local_idx);
            local_idx += 1;
        }

        // Dedicated local for frame pointer ($fp) if shadow stack frame > 0
        let fp_local = if func.frame_size > 0 {
            let l = local_idx;
            local_idx += 1;
            Some(l)
        } else {
            None
        };

        // Allocate remaining virtual registers to typed locals
        let mut i32_locals = 0u32;
        let mut i64_locals = 0u32;
        let mut f32_locals = 0u32;
        let mut f64_locals = 0u32;

        let mut internal_vregs: Vec<(VReg, MirType)> = func
            .vreg_types
            .iter()
            .filter(|(vr, _)| !vreg_to_local.contains_key(vr))
            .map(|(&vr, &ty)| (vr, ty))
            .collect();
        internal_vregs.sort_by_key(|(vr, _)| vr.0);

        for (vr, ty) in internal_vregs {
            vreg_to_local.insert(vr, local_idx);
            local_idx += 1;
            match ty {
                MirType::I32 => i32_locals += 1,
                MirType::I64 => i64_locals += 1,
                MirType::F32 => f32_locals += 1,
                MirType::F64 => f64_locals += 1,
                MirType::Void => {}
            }
        }

        let mut local_declarations = Vec::new();
        if let Some(_) = fp_local {
            i32_locals += 1;
        }
        if i32_locals > 0 {
            local_declarations.push((i32_locals, WasmValType::I32));
        }
        if i64_locals > 0 {
            local_declarations.push((i64_locals, WasmValType::I64));
        }
        if f32_locals > 0 {
            local_declarations.push((f32_locals, WasmValType::F32));
        }
        if f64_locals > 0 {
            local_declarations.push((f64_locals, WasmValType::F64));
        }

        let mut opcodes = Vec::new();

        // Function Prologue (Section 4.4, ADR-006): Shadow Stack allocation & Overflow check
        if func.frame_size > 0 {
            let fp = fp_local.unwrap();
            opcodes.push(WasmOpcode::GlobalGet(self.stack_pointer_global));
            opcodes.push(WasmOpcode::I32Const(func.frame_size as i32));
            opcodes.push(WasmOpcode::I32Sub);
            opcodes.push(WasmOpcode::LocalTee(fp));
            opcodes.push(WasmOpcode::GlobalSet(self.stack_pointer_global));

            if self.emit_stack_check {
                opcodes.push(WasmOpcode::LocalGet(fp));
                opcodes.push(WasmOpcode::I32Const(self.data_end_offset as i32));
                opcodes.push(WasmOpcode::I32LtU);
                opcodes.push(WasmOpcode::If(BlockType::Empty));
                opcodes.push(WasmOpcode::Unreachable);
                opcodes.push(WasmOpcode::End);
            }
        }

        // Lower Structured Instructions
        for inst in &func.instructions {
            match inst {
                StructuredInst::BlockStart => opcodes.push(WasmOpcode::Block(BlockType::Empty)),
                StructuredInst::LoopStart => opcodes.push(WasmOpcode::Loop(BlockType::Empty)),
                StructuredInst::End => opcodes.push(WasmOpcode::End),
                StructuredInst::Br(depth) => opcodes.push(WasmOpcode::Br(*depth)),
                StructuredInst::BrIf(depth, cond) => {
                    let loc = vreg_to_local[cond];
                    opcodes.push(WasmOpcode::LocalGet(loc));
                    opcodes.push(WasmOpcode::BrIf(*depth));
                }
                StructuredInst::Mir(mir) => {
                    self.lower_mir_inst(mir, &vreg_to_local, fp_local, &mut opcodes);
                }
                StructuredInst::Return(ret_val) => {
                    // Function Epilogue: Restore shadow stack pointer
                    if func.frame_size > 0 {
                        let fp = fp_local.unwrap();
                        opcodes.push(WasmOpcode::LocalGet(fp));
                        opcodes.push(WasmOpcode::I32Const(func.frame_size as i32));
                        opcodes.push(WasmOpcode::I32Add);
                        opcodes.push(WasmOpcode::GlobalSet(self.stack_pointer_global));
                    }
                    if let Some(vr) = ret_val {
                        let loc = vreg_to_local[vr];
                        opcodes.push(WasmOpcode::LocalGet(loc));
                    }
                    opcodes.push(WasmOpcode::Return);
                }
                StructuredInst::TailCall { func_name, args } => {
                    if func.frame_size > 0 {
                        let fp = fp_local.unwrap();
                        opcodes.push(WasmOpcode::LocalGet(fp));
                        opcodes.push(WasmOpcode::I32Const(func.frame_size as i32));
                        opcodes.push(WasmOpcode::I32Add);
                        opcodes.push(WasmOpcode::GlobalSet(self.stack_pointer_global));
                    }
                    for a in args {
                        let loc = vreg_to_local[a];
                        opcodes.push(WasmOpcode::LocalGet(loc));
                    }
                    let f_idx = self.func_indices.get(func_name).copied().unwrap_or(0);
                    opcodes.push(WasmOpcode::ReturnCall(f_idx));
                }
                StructuredInst::Throw(vr) => {
                    let loc = vreg_to_local[vr];
                    opcodes.push(WasmOpcode::LocalGet(loc));
                    opcodes.push(WasmOpcode::Throw(0));
                }
                StructuredInst::Unreachable => opcodes.push(WasmOpcode::Unreachable),
            }
        }

        // Implicit epilogue if function flows to the end
        if func.frame_size > 0 {
            let fp = fp_local.unwrap();
            opcodes.push(WasmOpcode::LocalGet(fp));
            opcodes.push(WasmOpcode::I32Const(func.frame_size as i32));
            opcodes.push(WasmOpcode::I32Add);
            opcodes.push(WasmOpcode::GlobalSet(self.stack_pointer_global));
        }

        if func.return_type != MirType::Void {
            if opcodes.last() != Some(&WasmOpcode::Return) && opcodes.last() != Some(&WasmOpcode::Unreachable) {
                opcodes.push(WasmOpcode::Unreachable);
            }
        }

        opcodes.push(WasmOpcode::End);

        // Peephole optimization: fuse LocalSet + LocalGet -> LocalTee
        let optimized_opcodes = Self::peephole_optimize(opcodes);

        CompiledFunction {
            name: func.name,
            type_index,
            locals: local_declarations,
            body_opcodes: optimized_opcodes,
        }
    }

    fn lower_mir_inst(
        &self,
        mir: &MirInst,
        vreg_map: &HashMap<VReg, u32>,
        fp_local: Option<u32>,
        out: &mut Vec<WasmOpcode>,
    ) {
        match mir {
            MirInst::ConstI32(dest, val) => {
                let loc = vreg_map[dest];
                out.push(WasmOpcode::I32Const(*val));
                out.push(WasmOpcode::LocalSet(loc));
            }
            MirInst::ConstI64(dest, val) => {
                let loc = vreg_map[dest];
                out.push(WasmOpcode::I64Const(*val));
                out.push(WasmOpcode::LocalSet(loc));
            }
            MirInst::ConstF32(dest, val) => {
                let loc = vreg_map[dest];
                out.push(WasmOpcode::F32Const(*val));
                out.push(WasmOpcode::LocalSet(loc));
            }
            MirInst::ConstF64(dest, val) => {
                let loc = vreg_map[dest];
                out.push(WasmOpcode::F64Const(*val));
                out.push(WasmOpcode::LocalSet(loc));
            }
            MirInst::Binary { dest, op, lhs, rhs } => {
                let l_loc = vreg_map[lhs];
                let r_loc = vreg_map[rhs];
                let d_loc = vreg_map[dest];
                out.push(WasmOpcode::LocalGet(l_loc));
                out.push(WasmOpcode::LocalGet(r_loc));

                let opc = match op {
                    MirBinOp::Add => WasmOpcode::I32Add,
                    MirBinOp::Sub => WasmOpcode::I32Sub,
                    MirBinOp::Mul => WasmOpcode::I32Mul,
                    MirBinOp::DivS => WasmOpcode::I32DivS,
                    MirBinOp::DivU => WasmOpcode::I32DivU,
                    MirBinOp::RemS => WasmOpcode::I32RemS,
                    MirBinOp::RemU => WasmOpcode::I32RemU,
                    MirBinOp::And => WasmOpcode::I32And,
                    MirBinOp::Or => WasmOpcode::I32Or,
                    MirBinOp::Xor => WasmOpcode::I32Xor,
                    MirBinOp::Shl => WasmOpcode::I32Shl,
                    MirBinOp::ShrS => WasmOpcode::I32ShrS,
                    MirBinOp::ShrU => WasmOpcode::I32ShrU,
                    MirBinOp::Eq => WasmOpcode::I32Eq,
                    MirBinOp::Ne => WasmOpcode::I32Ne,
                    MirBinOp::LtS => WasmOpcode::I32LtS,
                    MirBinOp::LtU => WasmOpcode::I32LtU,
                    MirBinOp::LeS => WasmOpcode::I32LeS,
                    MirBinOp::LeU => WasmOpcode::I32LeU,
                    MirBinOp::GtS => WasmOpcode::I32GtS,
                    MirBinOp::GtU => WasmOpcode::I32GtU,
                    MirBinOp::GeS => WasmOpcode::I32GeS,
                    MirBinOp::GeU => WasmOpcode::I32GeU,
                    MirBinOp::FAdd => WasmOpcode::F64Add,
                    MirBinOp::FSub => WasmOpcode::F64Sub,
                    MirBinOp::FMul => WasmOpcode::F64Mul,
                    MirBinOp::FDiv => WasmOpcode::F64Div,
                    MirBinOp::FEq => WasmOpcode::F64Eq,
                    MirBinOp::FNe => WasmOpcode::F64Ne,
                    MirBinOp::FLt => WasmOpcode::F64Lt,
                    MirBinOp::FLe => WasmOpcode::F64Le,
                    MirBinOp::FGt => WasmOpcode::F64Gt,
                    MirBinOp::FGe => WasmOpcode::F64Ge,
                };
                out.push(opc);
                out.push(WasmOpcode::LocalSet(d_loc));
            }
            MirInst::Unary { dest, op, operand } => {
                let opnd_loc = vreg_map[operand];
                let d_loc = vreg_map[dest];
                out.push(WasmOpcode::LocalGet(opnd_loc));
                match op {
                    MirUnOp::Eqz => out.push(WasmOpcode::I32Eqz),
                    MirUnOp::FNeg => out.push(WasmOpcode::F64Neg),
                }
                out.push(WasmOpcode::LocalSet(d_loc));
            }
            MirInst::Load { dest, ty, ptr, offset, align } => {
                let p_loc = vreg_map[ptr];
                let d_loc = vreg_map[dest];
                out.push(WasmOpcode::LocalGet(p_loc));
                let opc = match ty {
                    MirType::I32 => WasmOpcode::I32Load { align: *align, offset: *offset },
                    MirType::I64 => WasmOpcode::I64Load { align: *align, offset: *offset },
                    MirType::F32 => WasmOpcode::F32Load { align: *align, offset: *offset },
                    MirType::F64 => WasmOpcode::F64Load { align: *align, offset: *offset },
                    MirType::Void => WasmOpcode::I32Load { align: *align, offset: *offset },
                };
                out.push(opc);
                out.push(WasmOpcode::LocalSet(d_loc));
            }
            MirInst::Store { ty, ptr, val, offset, align } => {
                let p_loc = vreg_map[ptr];
                let v_loc = vreg_map[val];
                out.push(WasmOpcode::LocalGet(p_loc));
                out.push(WasmOpcode::LocalGet(v_loc));
                let opc = match ty {
                    MirType::I32 => WasmOpcode::I32Store { align: *align, offset: *offset },
                    MirType::I64 => WasmOpcode::I64Store { align: *align, offset: *offset },
                    MirType::F32 => WasmOpcode::F32Store { align: *align, offset: *offset },
                    MirType::F64 => WasmOpcode::F64Store { align: *align, offset: *offset },
                    MirType::Void => WasmOpcode::I32Store { align: *align, offset: *offset },
                };
                out.push(opc);
            }
            MirInst::GlobalGet { dest, global_idx } => {
                let d_loc = vreg_map[dest];
                out.push(WasmOpcode::GlobalGet(*global_idx));
                out.push(WasmOpcode::LocalSet(d_loc));
            }
            MirInst::GlobalSet { global_idx, val } => {
                let v_loc = vreg_map[val];
                out.push(WasmOpcode::LocalGet(v_loc));
                out.push(WasmOpcode::GlobalSet(*global_idx));
            }
            MirInst::StackAddr { dest, frame_offset } => {
                let d_loc = vreg_map[dest];
                if let Some(fp) = fp_local {
                    out.push(WasmOpcode::LocalGet(fp));
                    out.push(WasmOpcode::I32Const(*frame_offset));
                    out.push(WasmOpcode::I32Add);
                    out.push(WasmOpcode::LocalSet(d_loc));
                } else {
                    out.push(WasmOpcode::I32Const(*frame_offset));
                    out.push(WasmOpcode::LocalSet(d_loc));
                }
            }
            MirInst::Call { dest, func_name, args } => {
                for a in args {
                    let loc = vreg_map[a];
                    out.push(WasmOpcode::LocalGet(loc));
                }
                let f_idx = self.func_indices.get(func_name).copied().unwrap_or(0);
                out.push(WasmOpcode::Call(f_idx));
                if let Some(d) = dest {
                    let d_loc = vreg_map[d];
                    out.push(WasmOpcode::LocalSet(d_loc));
                }
            }
            MirInst::CallIndirect { dest, table_idx, args } => {
                for a in args {
                    let loc = vreg_map[a];
                    out.push(WasmOpcode::LocalGet(loc));
                }
                let tbl_loc = vreg_map[table_idx];
                out.push(WasmOpcode::LocalGet(tbl_loc));
                out.push(WasmOpcode::CallIndirect { type_idx: 0, table_idx: 0 });
                if let Some(d) = dest {
                    let d_loc = vreg_map[d];
                    out.push(WasmOpcode::LocalSet(d_loc));
                }
            }
            MirInst::Cast { dest, from, to, val } => {
                let v_loc = vreg_map[val];
                let d_loc = vreg_map[dest];
                out.push(WasmOpcode::LocalGet(v_loc));
                match (from, to) {
                    (MirType::F64, MirType::I32) => out.push(WasmOpcode::I32TruncSatF64S),
                    (MirType::I32, MirType::F64) => out.push(WasmOpcode::F64ConvertI32S),
                    (MirType::F32, MirType::I32) => out.push(WasmOpcode::I32TruncSatF32S),
                    (MirType::I32, MirType::F32) => out.push(WasmOpcode::F32ConvertI32S),
                    _ => {}
                }
                out.push(WasmOpcode::LocalSet(d_loc));
            }
            MirInst::Copy { dest, src } => {
                let s_loc = vreg_map[src];
                let d_loc = vreg_map[dest];
                out.push(WasmOpcode::LocalGet(s_loc));
                out.push(WasmOpcode::LocalSet(d_loc));
            }
        }
    }

    fn peephole_optimize(opcodes: Vec<WasmOpcode>) -> Vec<WasmOpcode> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < opcodes.len() {
            if i + 1 < opcodes.len() {
                if let (WasmOpcode::LocalSet(a), WasmOpcode::LocalGet(b)) = (&opcodes[i], &opcodes[i + 1]) {
                    if a == b {
                        out.push(WasmOpcode::LocalTee(*a));
                        i += 2;
                        continue;
                    }
                }
            }
            out.push(opcodes[i].clone());
            i += 1;
        }
        out
    }
}
