use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use wasmc_ast::*;
use wasmc_hir::*;
use wasmc_lexer::SymbolId;
use wasmc_semantics::{SemType, StorageClass, SymbolEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VReg(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MirType {
    I32,
    I64,
    F32,
    F64,
    Void,
}

impl MirType {
    pub fn from_sem(ty: &SemType) -> Self {
        match ty {
            SemType::Void => MirType::Void,
            SemType::I32 | SemType::U32 | SemType::Bool | SemType::Char | SemType::Pointer(_) | SemType::Function { .. } => MirType::I32,
            SemType::I64 | SemType::U64 => MirType::I64,
            SemType::F32 => MirType::F32,
            SemType::F64 => MirType::F64,
            SemType::Array(_, _) | SemType::Struct(_) => MirType::I32, // address pointer
            SemType::Poison => MirType::I32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MirBinOp {
    Add, Sub, Mul, DivS, DivU, RemS, RemU,
    And, Or, Xor, Shl, ShrS, ShrU,
    Eq, Ne, LtS, LtU, LeS, LeU, GtS, GtU, GeS, GeU,
    FAdd, FSub, FMul, FDiv,
    FEq, FNe, FLt, FLe, FGt, FGe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MirUnOp {
    Eqz,
    FNeg,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MirInst {
    ConstI32(VReg, i32),
    ConstI64(VReg, i64),
    ConstF32(VReg, f32),
    ConstF64(VReg, f64),
    Binary {
        dest: VReg,
        op: MirBinOp,
        lhs: VReg,
        rhs: VReg,
    },
    Unary {
        dest: VReg,
        op: MirUnOp,
        operand: VReg,
    },
    Load {
        dest: VReg,
        ty: MirType,
        ptr: VReg,
        offset: u32,
        align: u32,
    },
    Store {
        ty: MirType,
        ptr: VReg,
        val: VReg,
        offset: u32,
        align: u32,
    },
    GlobalGet {
        dest: VReg,
        global_idx: u32,
    },
    GlobalSet {
        global_idx: u32,
        val: VReg,
    },
    StackAddr {
        dest: VReg,
        frame_offset: i32,
    },
    Call {
        dest: Option<VReg>,
        func_name: SymbolId,
        args: Vec<VReg>,
    },
    CallIndirect {
        dest: Option<VReg>,
        table_idx: VReg,
        args: Vec<VReg>,
    },
    Cast {
        dest: VReg,
        from: MirType,
        to: MirType,
        val: VReg,
    },
    Copy {
        dest: VReg,
        src: VReg,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhiNode {
    pub dest: VReg,
    pub ty: MirType,
    pub incoming: Vec<(BlockId, VReg)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Terminator {
    Jump(BlockId),
    Branch {
        cond: VReg,
        true_block: BlockId,
        false_block: BlockId,
    },
    Return(Option<VReg>),
    TailCall {
        func_name: SymbolId,
        args: Vec<VReg>,
    },
    Throw(VReg),
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BasicBlock {
    pub id: BlockId,
    pub phis: Vec<PhiNode>,
    pub instructions: Vec<MirInst>,
    pub terminator: Terminator,
    pub predecessors: Vec<BlockId>,
    pub successors: Vec<BlockId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirFunction {
    pub name: SymbolId,
    pub params: Vec<(VReg, MirType)>,
    pub return_type: MirType,
    pub entry_block: BlockId,
    pub blocks: Vec<BasicBlock>,
    pub vreg_types: HashMap<VReg, MirType>,
    pub frame_size: u32,
    pub locals: Vec<SymbolEntry>,
}

impl MirFunction {
    pub fn dump_text(&self, interner: &wasmc_lexer::SymbolInterner) -> String {
        let fn_name = interner.resolve(self.name).unwrap_or("anon");
        let mut out = format!("function @{}(", fn_name);
        for (i, (vr, ty)) in self.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("%v{}: {:?}", vr.0, ty));
        }
        out.push_str(&format!(") -> {:?} [frame_size={}] {{\n", self.return_type, self.frame_size));

        for block in &self.blocks {
            out.push_str(&format!("  block bb{}:\n", block.id.0));
            for phi in &block.phis {
                out.push_str(&format!("    %v{} = phi {:?} [", phi.dest.0, phi.ty));
                for (j, (b, v)) in phi.incoming.iter().enumerate() {
                    if j > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!("bb{}: %v{}", b.0, v.0));
                }
                out.push_str("]\n");
            }
            for inst in &block.instructions {
                out.push_str(&format!("    {:?}\n", inst));
            }
            out.push_str(&format!("    {:?}\n", block.terminator));
        }
        out.push_str("}\n");
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
    pub globals: Vec<SymbolEntry>,
}

// Braun et al. on-the-fly SSA builder
pub struct MirBuilder {
    vreg_counter: u32,
    block_counter: u32,
    current_block: BlockId,
    blocks: Vec<BasicBlock>,
    vreg_types: HashMap<VReg, MirType>,
    current_def: HashMap<(SymbolId, BlockId), VReg>,
    sealed_blocks: HashSet<BlockId>,
    incomplete_phis: HashMap<BlockId, Vec<(SymbolId, VReg, MirType)>>,
    loop_continue_targets: Vec<BlockId>,
    loop_break_targets: Vec<BlockId>,
}

impl MirBuilder {
    pub fn new() -> Self {
        let mut b = Self {
            vreg_counter: 0,
            block_counter: 0,
            current_block: BlockId(0),
            blocks: Vec::new(),
            vreg_types: HashMap::new(),
            current_def: HashMap::new(),
            sealed_blocks: HashSet::new(),
            incomplete_phis: HashMap::new(),
            loop_continue_targets: Vec::new(),
            loop_break_targets: Vec::new(),
        };
        let entry = b.create_block();
        b.current_block = entry;
        b
    }

    pub fn new_vreg(&mut self, ty: MirType) -> VReg {
        let v = VReg(self.vreg_counter);
        self.vreg_counter += 1;
        self.vreg_types.insert(v, ty);
        v
    }

    pub fn create_block(&mut self) -> BlockId {
        let b = BlockId(self.block_counter);
        self.block_counter += 1;
        self.blocks.push(BasicBlock {
            id: b,
            phis: Vec::new(),
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            predecessors: Vec::new(),
            successors: Vec::new(),
        });
        b
    }

    pub fn set_current_block(&mut self, block: BlockId) {
        self.current_block = block;
    }

    pub fn emit(&mut self, inst: MirInst) {
        let b = &mut self.blocks[self.current_block.0 as usize];
        b.instructions.push(inst);
    }

    pub fn terminate(&mut self, term: Terminator) {
        let cur = self.current_block;
        match &term {
            Terminator::Jump(target) => {
                let target = *target;
                self.blocks[cur.0 as usize].successors.push(target);
                self.blocks[target.0 as usize].predecessors.push(cur);
            }
            Terminator::Branch { true_block, false_block, .. } => {
                let tb = *true_block;
                let fb = *false_block;
                self.blocks[cur.0 as usize].successors.push(fb);
                self.blocks[cur.0 as usize].successors.push(tb);
                self.blocks[tb.0 as usize].predecessors.push(cur);
                self.blocks[fb.0 as usize].predecessors.push(cur);
            }
            _ => {}
        }
        self.blocks[cur.0 as usize].terminator = term;
    }

    pub fn write_variable(&mut self, variable: SymbolId, block: BlockId, value: VReg) {
        self.current_def.insert((variable, block), value);
    }

    pub fn read_variable(&mut self, variable: SymbolId, block: BlockId, ty: MirType) -> VReg {
        if let Some(&val) = self.current_def.get(&(variable, block)) {
            val
        } else {
            self.read_variable_recursive(variable, block, ty)
        }
    }

    fn read_variable_recursive(&mut self, variable: SymbolId, block: BlockId, ty: MirType) -> VReg {
        let val = if !self.sealed_blocks.contains(&block) {
            let phi_dest = self.new_vreg(ty);
            self.incomplete_phis
                .entry(block)
                .or_default()
                .push((variable, phi_dest, ty));
            phi_dest
        } else {
            let preds = self.blocks[block.0 as usize].predecessors.clone();
            if preds.len() == 1 {
                self.read_variable(variable, preds[0], ty)
            } else {
                let phi_dest = self.new_vreg(ty);
                self.write_variable(variable, block, phi_dest);
                self.add_phi_operands(variable, phi_dest, block, ty);
                phi_dest
            }
        };
        self.write_variable(variable, block, val);
        val
    }

    fn add_phi_operands(&mut self, variable: SymbolId, phi_dest: VReg, block: BlockId, ty: MirType) {
        let preds = self.blocks[block.0 as usize].predecessors.clone();
        let mut incoming = Vec::new();
        for pred in preds {
            let v = self.read_variable(variable, pred, ty);
            incoming.push((pred, v));
        }
        self.blocks[block.0 as usize].phis.push(PhiNode {
            dest: phi_dest,
            ty,
            incoming,
        });
    }

    pub fn seal_block(&mut self, block: BlockId) {
        if self.sealed_blocks.insert(block) {
            if let Some(incompletes) = self.incomplete_phis.remove(&block) {
                for (var, phi_dest, ty) in incompletes {
                    self.add_phi_operands(var, phi_dest, block, ty);
                }
            }
        }
    }
}

pub fn lower_hir_to_mir(hir: HirModule) -> MirModule {
    let mut mir_functions = Vec::new();

    for f in hir.functions {
        let mut builder = MirBuilder::new();
        let ret_ty = MirType::from_sem(&f.return_type);

        let mut param_vregs = Vec::new();
        builder.seal_block(BlockId(0));

        for p in &f.params {
            let p_ty = MirType::from_sem(&p.sem_type);
            let vr = builder.new_vreg(p_ty);
            param_vregs.push((vr, p_ty));
            builder.write_variable(p.name, BlockId(0), vr);
        }

        lower_hir_stmt(&f.body, &mut builder, ret_ty);

        // Ensure current block (and any unterminated block falling through in a void function) has a return terminator
        let cur = builder.current_block.0 as usize;
        if cur < builder.blocks.len() && builder.blocks[cur].terminator == Terminator::Unreachable {
            if ret_ty == MirType::Void {
                builder.blocks[cur].terminator = Terminator::Return(None);
            }
        }

        // Ensure every block has a terminator
        for block in &mut builder.blocks {
            if block.terminator == Terminator::Unreachable && block.instructions.is_empty() && block.id != BlockId(0) {
                block.terminator = if ret_ty == MirType::Void {
                    Terminator::Return(None)
                } else {
                    Terminator::Unreachable
                };
            }
        }

        mir_functions.push(MirFunction {
            name: f.name,
            params: param_vregs,
            return_type: ret_ty,
            entry_block: BlockId(0),
            blocks: builder.blocks,
            vreg_types: builder.vreg_types,
            frame_size: f.frame_size,
            locals: f.locals,
        });
    }

    MirModule {
        functions: mir_functions,
        globals: hir.globals,
    }
}

fn lower_hir_stmt(stmt: &HirStmt, b: &mut MirBuilder, fn_ret_ty: MirType) {
    match stmt {
        HirStmt::Block(stmts) => {
            for s in stmts {
                lower_hir_stmt(s, b, fn_ret_ty);
            }
        }
        HirStmt::AssignVar(name, storage, expr) => {
            let val = lower_hir_expr(expr, b);
            match storage {
                StorageClass::VirtualLocal { .. } => {
                    b.write_variable(*name, b.current_block, val);
                }
                StorageClass::GlobalSlot { global_idx, .. } => {
                    b.emit(MirInst::GlobalSet {
                        global_idx: *global_idx,
                        val,
                    });
                }
                StorageClass::ShadowStack { frame_offset, align_bytes, .. } => {
                    let ptr_reg = b.new_vreg(MirType::I32);
                    b.emit(MirInst::StackAddr {
                        dest: ptr_reg,
                        frame_offset: *frame_offset,
                    });
                    let ty = MirType::from_sem(expr.ty());
                    b.emit(MirInst::Store {
                        ty,
                        ptr: ptr_reg,
                        val,
                        offset: 0,
                        align: *align_bytes,
                    });
                }
                _ => {}
            }
        }
        HirStmt::Store { ptr, val, offset, align } => {
            let p_reg = lower_hir_expr(ptr, b);
            let v_reg = lower_hir_expr(val, b);
            let ty = MirType::from_sem(val.ty());
            b.emit(MirInst::Store {
                ty,
                ptr: p_reg,
                val: v_reg,
                offset: *offset,
                align: *align,
            });
        }
        HirStmt::If { cond, then_branch, else_branch } => {
            let cond_reg = lower_hir_expr(cond, b);
            let then_bb = b.create_block();
            let merge_bb = b.create_block();
            let else_bb = if else_branch.is_some() {
                b.create_block()
            } else {
                merge_bb
            };

            b.terminate(Terminator::Branch {
                cond: cond_reg,
                true_block: then_bb,
                false_block: else_bb,
            });

            // Then block
            b.set_current_block(then_bb);
            b.seal_block(then_bb);
            lower_hir_stmt(then_branch, b, fn_ret_ty);
            if b.blocks[b.current_block.0 as usize].terminator == Terminator::Unreachable {
                b.terminate(Terminator::Jump(merge_bb));
            }

            // Else block
            if let Some(eb) = else_branch {
                b.set_current_block(else_bb);
                b.seal_block(else_bb);
                lower_hir_stmt(eb, b, fn_ret_ty);
                if b.blocks[b.current_block.0 as usize].terminator == Terminator::Unreachable {
                    b.terminate(Terminator::Jump(merge_bb));
                }
            }

            // Merge block
            b.set_current_block(merge_bb);
            b.seal_block(merge_bb);
        }
        HirStmt::While { cond, body } => {
            let header_bb = b.create_block();
            let body_bb = b.create_block();
            let exit_bb = b.create_block();

            b.terminate(Terminator::Jump(header_bb));

            b.set_current_block(header_bb);
            let cond_reg = lower_hir_expr(cond, b);
            b.terminate(Terminator::Branch {
                cond: cond_reg,
                true_block: body_bb,
                false_block: exit_bb,
            });

            b.loop_continue_targets.push(header_bb);
            b.loop_break_targets.push(exit_bb);

            b.set_current_block(body_bb);
            b.seal_block(body_bb);
            lower_hir_stmt(body, b, fn_ret_ty);
            if b.blocks[b.current_block.0 as usize].terminator == Terminator::Unreachable {
                b.terminate(Terminator::Jump(header_bb));
            }

            b.loop_continue_targets.pop();
            b.loop_break_targets.pop();

            b.seal_block(header_bb);

            b.set_current_block(exit_bb);
            b.seal_block(exit_bb);
        }
        HirStmt::Return { is_tail, value } => {
            let v = value.as_ref().map(|val| lower_hir_expr(val, b));
            if *is_tail {
                if let Some(HirExpr::Call(callee, args, _)) = value {
                    let arg_regs = args.iter().map(|a| lower_hir_expr(a, b)).collect();
                    b.terminate(Terminator::TailCall {
                        func_name: *callee,
                        args: arg_regs,
                    });
                    return;
                }
            }
            b.terminate(Terminator::Return(v));
        }
        HirStmt::Break => {
            if let Some(&target) = b.loop_break_targets.last() {
                b.terminate(Terminator::Jump(target));
            }
        }
        HirStmt::Continue => {
            if let Some(&target) = b.loop_continue_targets.last() {
                b.terminate(Terminator::Jump(target));
            }
        }
        HirStmt::Throw(e) => {
            let val = lower_hir_expr(e, b);
            b.terminate(Terminator::Throw(val));
        }
        HirStmt::TryCatch { try_body, err_name, catch_body } => {
            let try_bb = b.create_block();
            let catch_bb = b.create_block();
            let merge_bb = b.create_block();

            b.terminate(Terminator::Jump(try_bb));

            b.set_current_block(try_bb);
            b.seal_block(try_bb);
            lower_hir_stmt(try_body, b, fn_ret_ty);
            if b.blocks[b.current_block.0 as usize].terminator == Terminator::Unreachable {
                b.terminate(Terminator::Jump(merge_bb));
            }

            b.set_current_block(catch_bb);
            b.seal_block(catch_bb);
            let err_reg = b.new_vreg(MirType::I32);
            b.write_variable(*err_name, catch_bb, err_reg);
            lower_hir_stmt(catch_body, b, fn_ret_ty);
            if b.blocks[b.current_block.0 as usize].terminator == Terminator::Unreachable {
                b.terminate(Terminator::Jump(merge_bb));
            }

            b.set_current_block(merge_bb);
            b.seal_block(merge_bb);
        }
        HirStmt::Expr(e) => {
            lower_hir_expr(e, b);
        }
    }
}

fn lower_hir_expr(expr: &HirExpr, b: &mut MirBuilder) -> VReg {
    let ty = MirType::from_sem(expr.ty());
    match expr {
        HirExpr::Literal(lit, _) => {
            let dest = b.new_vreg(ty);
            match lit {
                LiteralValue::Integer(n) => {
                    if ty == MirType::I64 {
                        b.emit(MirInst::ConstI64(dest, *n));
                    } else {
                        b.emit(MirInst::ConstI32(dest, *n as i32));
                    }
                }
                LiteralValue::Float(f) => {
                    if ty == MirType::F32 {
                        b.emit(MirInst::ConstF32(dest, *f as f32));
                    } else {
                        b.emit(MirInst::ConstF64(dest, *f));
                    }
                }
                LiteralValue::Bool(bv) => {
                    b.emit(MirInst::ConstI32(dest, if *bv { 1 } else { 0 }));
                }
                LiteralValue::Char(c) => {
                    b.emit(MirInst::ConstI32(dest, *c as i32));
                }
                LiteralValue::String(sym) => {
                    b.emit(MirInst::ConstI32(dest, (1024 + sym.0 * 64) as i32));
                }
            }
            dest
        }
        HirExpr::Variable(sym, _sem_ty, storage) => {
            match storage {
                StorageClass::VirtualLocal { .. } => {
                    b.read_variable(*sym, b.current_block, ty)
                }
                StorageClass::GlobalSlot { global_idx, .. } => {
                    let dest = b.new_vreg(ty);
                    b.emit(MirInst::GlobalGet {
                        dest,
                        global_idx: *global_idx,
                    });
                    dest
                }
                StorageClass::ShadowStack { frame_offset, align_bytes, .. } => {
                    let ptr_reg = b.new_vreg(MirType::I32);
                    b.emit(MirInst::StackAddr {
                        dest: ptr_reg,
                        frame_offset: *frame_offset,
                    });
                    let dest = b.new_vreg(ty);
                    b.emit(MirInst::Load {
                        dest,
                        ty,
                        ptr: ptr_reg,
                        offset: 0,
                        align: *align_bytes,
                    });
                    dest
                }
                StorageClass::FunctionRef { func_idx } => {
                    let dest = b.new_vreg(MirType::I32);
                    b.emit(MirInst::ConstI32(dest, *func_idx as i32));
                    dest
                }
            }
        }
        HirExpr::Unary(op, inner, _) => {
            let inner_reg = lower_hir_expr(inner, b);
            let dest = b.new_vreg(ty);
            match op {
                UnaryOp::Neg => {
                    if ty == MirType::F64 || ty == MirType::F32 {
                        b.emit(MirInst::Unary {
                            dest,
                            op: MirUnOp::FNeg,
                            operand: inner_reg,
                        });
                    } else {
                        let zero = b.new_vreg(ty);
                        b.emit(MirInst::ConstI32(zero, 0));
                        b.emit(MirInst::Binary {
                            dest,
                            op: MirBinOp::Sub,
                            lhs: zero,
                            rhs: inner_reg,
                        });
                    }
                }
                UnaryOp::Not => {
                    b.emit(MirInst::Unary {
                        dest,
                        op: MirUnOp::Eqz,
                        operand: inner_reg,
                    });
                }
                UnaryOp::BitNot => {
                    let minus_one = b.new_vreg(ty);
                    b.emit(MirInst::ConstI32(minus_one, -1));
                    b.emit(MirInst::Binary {
                        dest,
                        op: MirBinOp::Xor,
                        lhs: inner_reg,
                        rhs: minus_one,
                    });
                }
                _ => return inner_reg,
            }
            dest
        }
        HirExpr::Binary(op, lhs, rhs, _) => {
            let l_reg = lower_hir_expr(lhs, b);
            let r_reg = lower_hir_expr(rhs, b);
            let dest = b.new_vreg(ty);

            let is_float = MirType::from_sem(lhs.ty()) == MirType::F64 || MirType::from_sem(lhs.ty()) == MirType::F32;

            let mir_op = match (op, is_float) {
                (BinaryOp::Add, false) => MirBinOp::Add,
                (BinaryOp::Add, true) => MirBinOp::FAdd,
                (BinaryOp::Sub, false) => MirBinOp::Sub,
                (BinaryOp::Sub, true) => MirBinOp::FSub,
                (BinaryOp::Mul, false) => MirBinOp::Mul,
                (BinaryOp::Mul, true) => MirBinOp::FMul,
                (BinaryOp::Div, false) => MirBinOp::DivS,
                (BinaryOp::Div, true) => MirBinOp::FDiv,
                (BinaryOp::Rem, _) => MirBinOp::RemS,
                (BinaryOp::BitAnd, _) => MirBinOp::And,
                (BinaryOp::BitOr, _) => MirBinOp::Or,
                (BinaryOp::BitXor, _) => MirBinOp::Xor,
                (BinaryOp::Shl, _) => MirBinOp::Shl,
                (BinaryOp::Shr, _) => MirBinOp::ShrS,
                (BinaryOp::Eq, false) => MirBinOp::Eq,
                (BinaryOp::Eq, true) => MirBinOp::FEq,
                (BinaryOp::Ne, false) => MirBinOp::Ne,
                (BinaryOp::Ne, true) => MirBinOp::FNe,
                (BinaryOp::Lt, false) => MirBinOp::LtS,
                (BinaryOp::Lt, true) => MirBinOp::FLt,
                (BinaryOp::Le, false) => MirBinOp::LeS,
                (BinaryOp::Le, true) => MirBinOp::FLe,
                (BinaryOp::Gt, false) => MirBinOp::GtS,
                (BinaryOp::Gt, true) => MirBinOp::FGt,
                (BinaryOp::Ge, false) => MirBinOp::GeS,
                (BinaryOp::Ge, true) => MirBinOp::FGe,
                (BinaryOp::LogicAnd, _) => MirBinOp::And,
                (BinaryOp::LogicOr, _) => MirBinOp::Or,
            };

            b.emit(MirInst::Binary {
                dest,
                op: mir_op,
                lhs: l_reg,
                rhs: r_reg,
            });
            dest
        }
        HirExpr::Call(callee, args, _) => {
            let arg_regs: Vec<VReg> = args.iter().map(|a| lower_hir_expr(a, b)).collect();
            let dest = if ty != MirType::Void {
                Some(b.new_vreg(ty))
            } else {
                None
            };
            b.emit(MirInst::Call {
                dest,
                func_name: *callee,
                args: arg_regs,
            });
            dest.unwrap_or_else(|| b.new_vreg(MirType::Void))
        }
        HirExpr::CallIndirect(target, args, _) => {
            let tbl_reg = lower_hir_expr(target, b);
            let arg_regs: Vec<VReg> = args.iter().map(|a| lower_hir_expr(a, b)).collect();
            let dest = if ty != MirType::Void {
                Some(b.new_vreg(ty))
            } else {
                None
            };
            b.emit(MirInst::CallIndirect {
                dest,
                table_idx: tbl_reg,
                args: arg_regs,
            });
            dest.unwrap_or_else(|| b.new_vreg(MirType::Void))
        }
        HirExpr::Load { ptr, offset, align, ty: load_sem_ty } => {
            let p_reg = lower_hir_expr(ptr, b);
            let dest = b.new_vreg(ty);
            b.emit(MirInst::Load {
                dest,
                ty: MirType::from_sem(load_sem_ty),
                ptr: p_reg,
                offset: *offset,
                align: *align,
            });
            dest
        }
        HirExpr::AddrOf(_, storage, _) => {
            let dest = b.new_vreg(MirType::I32);
            match storage {
                StorageClass::ShadowStack { frame_offset, .. } => {
                    b.emit(MirInst::StackAddr {
                        dest,
                        frame_offset: *frame_offset,
                    });
                }
                _ => {
                    b.emit(MirInst::ConstI32(dest, 1024));
                }
            }
            dest
        }
        HirExpr::Cast(inner, _) => {
            let in_reg = lower_hir_expr(inner, b);
            let from_ty = MirType::from_sem(inner.ty());
            let dest = b.new_vreg(ty);
            b.emit(MirInst::Cast {
                dest,
                from: from_ty,
                to: ty,
                val: in_reg,
            });
            dest
        }
    }
}
