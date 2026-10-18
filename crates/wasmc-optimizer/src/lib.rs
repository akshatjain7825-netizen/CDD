use std::collections::{HashMap, HashSet};
use wasmc_mir::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptLevel {
    O0,
    O1,
    #[default]
    O2,
    O3,
    Os,
    Oz,
}

impl OptLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "0" | "O0" => Some(OptLevel::O0),
            "1" | "O1" => Some(OptLevel::O1),
            "2" | "O2" => Some(OptLevel::O2),
            "3" | "O3" => Some(OptLevel::O3),
            "s" | "Os" => Some(OptLevel::Os),
            "z" | "Oz" => Some(OptLevel::Oz),
            _ => None,
        }
    }
}

pub struct Optimizer {
    level: OptLevel,
}

impl Optimizer {
    pub fn new(level: OptLevel) -> Self {
        Self { level }
    }

    pub fn optimize_module(&self, mut module: MirModule) -> MirModule {
        if self.level == OptLevel::O0 {
            return module;
        }

        for func in &mut module.functions {
            self.optimize_function(func);
        }

        module
    }

    pub fn optimize_function(&self, func: &mut MirFunction) {
        if self.level == OptLevel::O0 {
            return;
        }

        // Bounded fixed-point loop (max 3 iterations until no IR mutations occur)
        let max_iters = match self.level {
            OptLevel::O0 => 0,
            OptLevel::O1 => 2,
            _ => 3,
        };

        for _ in 0..max_iters {
            let mut changed = false;
            // Constant Propagation -> Algebraic Simplification -> CSE -> Copy Propagation -> DCE
            changed |= self.pass_constant_folding(func);
            changed |= self.pass_algebraic_simplification(func);
            changed |= self.pass_cse(func);
            changed |= self.pass_copy_propagation(func);
            changed |= self.pass_dce(func);
            changed |= self.pass_prune_unreachable_blocks(func);
            if !changed {
                break;
            }
        }
    }

    fn pass_constant_folding(&self, func: &mut MirFunction) -> bool {
        let mut changed = false;
        let mut const_i32: HashMap<VReg, i32> = HashMap::new();
        let mut const_i64: HashMap<VReg, i64> = HashMap::new();
        let mut const_f32: HashMap<VReg, f32> = HashMap::new();
        let mut const_f64: HashMap<VReg, f64> = HashMap::new();

        // Pass 1: Collect constants
        for block in &func.blocks {
            for inst in &block.instructions {
                match inst {
                    MirInst::ConstI32(dest, val) => {
                        const_i32.insert(*dest, *val);
                    }
                    MirInst::ConstI64(dest, val) => {
                        const_i64.insert(*dest, *val);
                    }
                    MirInst::ConstF32(dest, val) => {
                        const_f32.insert(*dest, *val);
                    }
                    MirInst::ConstF64(dest, val) => {
                        const_f64.insert(*dest, *val);
                    }
                    MirInst::Copy { dest, src } => {
                        if let Some(&val) = const_i32.get(src) {
                            const_i32.insert(*dest, val);
                        } else if let Some(&val) = const_i64.get(src) {
                            const_i64.insert(*dest, val);
                        } else if let Some(&val) = const_f32.get(src) {
                            const_f32.insert(*dest, val);
                        } else if let Some(&val) = const_f64.get(src) {
                            const_f64.insert(*dest, val);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Pass 2: Fold constant binary/unary instructions
        for block in &mut func.blocks {
            for inst in &mut block.instructions {
                if let MirInst::Binary { dest, op, lhs, rhs } = *inst {
                    if let (Some(&l), Some(&r)) = (const_i32.get(&lhs), const_i32.get(&rhs)) {
                        let res = match op {
                            MirBinOp::Add => Some(l.wrapping_add(r)),
                            MirBinOp::Sub => Some(l.wrapping_sub(r)),
                            MirBinOp::Mul => Some(l.wrapping_mul(r)),
                            MirBinOp::DivS if r != 0 => Some(l.wrapping_div(r)),
                            MirBinOp::DivU if r != 0 => Some((l as u32 / r as u32) as i32),
                            MirBinOp::RemS if r != 0 => Some(l.wrapping_rem(r)),
                            MirBinOp::RemU if r != 0 => Some((l as u32 % r as u32) as i32),
                            MirBinOp::And => Some(l & r),
                            MirBinOp::Or => Some(l | r),
                            MirBinOp::Xor => Some(l ^ r),
                            MirBinOp::Shl => Some(l.wrapping_shl((r as u32) & 31)),
                            MirBinOp::ShrS => Some(l.wrapping_shr((r as u32) & 31)),
                            MirBinOp::ShrU => Some(((l as u32).wrapping_shr((r as u32) & 31)) as i32),
                            MirBinOp::Eq => Some(if l == r { 1 } else { 0 }),
                            MirBinOp::Ne => Some(if l != r { 1 } else { 0 }),
                            MirBinOp::LtS => Some(if l < r { 1 } else { 0 }),
                            MirBinOp::LeS => Some(if l <= r { 1 } else { 0 }),
                            MirBinOp::GtS => Some(if l > r { 1 } else { 0 }),
                            MirBinOp::GeS => Some(if l >= r { 1 } else { 0 }),
                            MirBinOp::LtU => Some(if (l as u32) < (r as u32) { 1 } else { 0 }),
                            MirBinOp::LeU => Some(if (l as u32) <= (r as u32) { 1 } else { 0 }),
                            MirBinOp::GtU => Some(if (l as u32) > (r as u32) { 1 } else { 0 }),
                            MirBinOp::GeU => Some(if (l as u32) >= (r as u32) { 1 } else { 0 }),
                            _ => None,
                        };
                        if let Some(val) = res {
                            *inst = MirInst::ConstI32(dest, val);
                            const_i32.insert(dest, val);
                            changed = true;
                        }
                    }
                }
            }

            // Fold constant conditional branches
            if let Terminator::Branch { cond, true_block, false_block } = &block.terminator {
                if let Some(&c) = const_i32.get(cond) {
                    let target = if c != 0 { *true_block } else { *false_block };
                    block.terminator = Terminator::Jump(target);
                    changed = true;
                }
            }
        }

        changed
    }

    fn pass_algebraic_simplification(&self, func: &mut MirFunction) -> bool {
        let mut changed = false;
        let mut const_i32: HashMap<VReg, i32> = HashMap::new();

        for block in &func.blocks {
            for inst in &block.instructions {
                if let MirInst::ConstI32(dest, val) = inst {
                    const_i32.insert(*dest, *val);
                }
            }
        }

        for block in &mut func.blocks {
            for inst in &mut block.instructions {
                if let MirInst::Binary { dest, op, lhs, rhs } = *inst {
                    // x + 0 -> x, x - 0 -> x
                    if (op == MirBinOp::Add || op == MirBinOp::Sub) && const_i32.get(&rhs) == Some(&0) {
                        *inst = MirInst::Copy { dest, src: lhs };
                        changed = true;
                    }
                    // 0 + x -> x
                    else if op == MirBinOp::Add && const_i32.get(&lhs) == Some(&0) {
                        *inst = MirInst::Copy { dest, src: rhs };
                        changed = true;
                    }
                    // x * 1 -> x
                    else if op == MirBinOp::Mul && const_i32.get(&rhs) == Some(&1) {
                        *inst = MirInst::Copy { dest, src: lhs };
                        changed = true;
                    }
                    // x * 0 -> 0
                    else if op == MirBinOp::Mul && const_i32.get(&rhs) == Some(&0) {
                        *inst = MirInst::ConstI32(dest, 0);
                        changed = true;
                    }
                    // Strength reduction: x * 2 -> x + x
                    else if op == MirBinOp::Mul && const_i32.get(&rhs) == Some(&2) {
                        *inst = MirInst::Binary {
                            dest,
                            op: MirBinOp::Add,
                            lhs,
                            rhs: lhs,
                        };
                        changed = true;
                    }
                }
            }
        }

        changed
    }

    fn pass_cse(&self, func: &mut MirFunction) -> bool {
        let mut changed = false;

        for block in &mut func.blocks {
            let mut value_map: HashMap<(MirBinOp, VReg, VReg), VReg> = HashMap::new();
            let mut stack_map: HashMap<i32, VReg> = HashMap::new();

            for inst in &mut block.instructions {
                match inst {
                    MirInst::Binary { dest, op, lhs, rhs } => {
                        let key = (*op, *lhs, *rhs);
                        if let Some(&prev_dest) = value_map.get(&key) {
                            *inst = MirInst::Copy { dest: *dest, src: prev_dest };
                            changed = true;
                        } else {
                            value_map.insert(key, *dest);
                        }
                    }
                    MirInst::StackAddr { dest, frame_offset } => {
                        if let Some(&prev_dest) = stack_map.get(frame_offset) {
                            *inst = MirInst::Copy { dest: *dest, src: prev_dest };
                            changed = true;
                        } else {
                            stack_map.insert(*frame_offset, *dest);
                        }
                    }
                    _ => {}
                }
            }
        }

        changed
    }

    fn pass_copy_propagation(&self, func: &mut MirFunction) -> bool {
        let mut changed = false;
        let mut copy_map: HashMap<VReg, VReg> = HashMap::new();

        for block in &func.blocks {
            for inst in &block.instructions {
                if let MirInst::Copy { dest, src } = inst {
                    if dest != src {
                        let mut resolved = *src;
                        while let Some(&next) = copy_map.get(&resolved) {
                            resolved = next;
                        }
                        copy_map.insert(*dest, resolved);
                    }
                }
            }
        }

        if copy_map.is_empty() {
            return false;
        }

        let resolve = |v: VReg| -> VReg {
            let mut curr = v;
            while let Some(&next) = copy_map.get(&curr) {
                curr = next;
            }
            curr
        };

        for block in &mut func.blocks {
            for phi in &mut block.phis {
                for (_, v) in &mut phi.incoming {
                    let new_v = resolve(*v);
                    if new_v != *v {
                        *v = new_v;
                        changed = true;
                    }
                }
            }

            for inst in &mut block.instructions {
                match inst {
                    MirInst::Binary { lhs, rhs, .. } => {
                        let new_l = resolve(*lhs);
                        let new_r = resolve(*rhs);
                        if new_l != *lhs { *lhs = new_l; changed = true; }
                        if new_r != *rhs { *rhs = new_r; changed = true; }
                    }
                    MirInst::Unary { operand, .. } => {
                        let new_op = resolve(*operand);
                        if new_op != *operand { *operand = new_op; changed = true; }
                    }
                    MirInst::Load { ptr, .. } => {
                        let new_ptr = resolve(*ptr);
                        if new_ptr != *ptr { *ptr = new_ptr; changed = true; }
                    }
                    MirInst::Store { ptr, val, .. } => {
                        let new_ptr = resolve(*ptr);
                        let new_val = resolve(*val);
                        if new_ptr != *ptr { *ptr = new_ptr; changed = true; }
                        if new_val != *val { *val = new_val; changed = true; }
                    }
                    MirInst::GlobalSet { val, .. } => {
                        let new_val = resolve(*val);
                        if new_val != *val { *val = new_val; changed = true; }
                    }
                    MirInst::Call { args, .. } => {
                        for a in args {
                            let new_a = resolve(*a);
                            if new_a != *a { *a = new_a; changed = true; }
                        }
                    }
                    MirInst::CallIndirect { table_idx, args, .. } => {
                        let new_t = resolve(*table_idx);
                        if new_t != *table_idx { *table_idx = new_t; changed = true; }
                        for a in args {
                            let new_a = resolve(*a);
                            if new_a != *a { *a = new_a; changed = true; }
                        }
                    }
                    MirInst::Cast { val, .. } => {
                        let new_val = resolve(*val);
                        if new_val != *val { *val = new_val; changed = true; }
                    }
                    MirInst::Copy { src, .. } => {
                        let new_src = resolve(*src);
                        if new_src != *src { *src = new_src; changed = true; }
                    }
                    _ => {}
                }
            }

            match &mut block.terminator {
                Terminator::Branch { cond, .. } => {
                    let new_c = resolve(*cond);
                    if new_c != *cond { *cond = new_c; changed = true; }
                }
                Terminator::Return(Some(vr)) => {
                    let new_vr = resolve(*vr);
                    if new_vr != *vr { *vr = new_vr; changed = true; }
                }
                Terminator::TailCall { args, .. } => {
                    for a in args {
                        let new_a = resolve(*a);
                        if new_a != *a { *a = new_a; changed = true; }
                    }
                }
                Terminator::Throw(vr) => {
                    let new_vr = resolve(*vr);
                    if new_vr != *vr { *vr = new_vr; changed = true; }
                }
                _ => {}
            }
        }

        changed
    }

    fn pass_dce(&self, func: &mut MirFunction) -> bool {
        let mut changed = false;

        // Step 1: Collect used registers
        let mut used: HashSet<VReg> = HashSet::new();

        for block in &func.blocks {
            for phi in &block.phis {
                for (_, v) in &phi.incoming {
                    used.insert(*v);
                }
            }
            for inst in &block.instructions {
                match inst {
                    MirInst::Binary { lhs, rhs, .. } => {
                        used.insert(*lhs);
                        used.insert(*rhs);
                    }
                    MirInst::Unary { operand, .. } => {
                        used.insert(*operand);
                    }
                    MirInst::Load { ptr, .. } => {
                        used.insert(*ptr);
                    }
                    MirInst::Store { ptr, val, .. } => {
                        used.insert(*ptr);
                        used.insert(*val);
                    }
                    MirInst::GlobalSet { val, .. } => {
                        used.insert(*val);
                    }
                    MirInst::Call { args, .. } => {
                        for a in args {
                            used.insert(*a);
                        }
                    }
                    MirInst::CallIndirect { table_idx, args, .. } => {
                        used.insert(*table_idx);
                        for a in args {
                            used.insert(*a);
                        }
                    }
                    MirInst::Cast { val, .. } => {
                        used.insert(*val);
                    }
                    MirInst::Copy { dest, src } => {
                        if dest != src {
                            used.insert(*src);
                        }
                    }
                    _ => {}
                }
            }
            match &block.terminator {
                Terminator::Branch { cond, .. } => {
                    used.insert(*cond);
                }
                Terminator::Return(Some(vr)) => {
                    used.insert(*vr);
                }
                Terminator::TailCall { args, .. } => {
                    for a in args {
                        used.insert(*a);
                    }
                }
                Terminator::Throw(vr) => {
                    used.insert(*vr);
                }
                _ => {}
            }
        }

        // Step 2: Remove instructions whose dest is not used (excluding side-effects like Store, Call, GlobalSet)
        for block in &mut func.blocks {
            let orig_len = block.instructions.len();
            block.instructions.retain(|inst| match inst {
                MirInst::ConstI32(dest, _)
                | MirInst::ConstI64(dest, _)
                | MirInst::ConstF32(dest, _)
                | MirInst::ConstF64(dest, _)
                | MirInst::Binary { dest, .. }
                | MirInst::Unary { dest, .. }
                | MirInst::GlobalGet { dest, .. }
                | MirInst::StackAddr { dest, .. }
                | MirInst::Cast { dest, .. } => used.contains(dest),
                MirInst::Copy { dest, src } => dest != src && used.contains(dest),
                _ => true, // Preserve side-effecting operations
            });
            if block.instructions.len() != orig_len {
                changed = true;
            }
        }

        changed
    }

    fn pass_prune_unreachable_blocks(&self, func: &mut MirFunction) -> bool {
        let mut reachable: HashSet<BlockId> = HashSet::new();
        let mut worklist = vec![func.entry_block];
        reachable.insert(func.entry_block);

        let block_map: HashMap<BlockId, &BasicBlock> = func.blocks.iter().map(|b| (b.id, b)).collect();

        while let Some(b_id) = worklist.pop() {
            if let Some(block) = block_map.get(&b_id) {
                match &block.terminator {
                    Terminator::Jump(target) => {
                        if reachable.insert(*target) {
                            worklist.push(*target);
                        }
                    }
                    Terminator::Branch { true_block, false_block, .. } => {
                        if reachable.insert(*true_block) {
                            worklist.push(*true_block);
                        }
                        if reachable.insert(*false_block) {
                            worklist.push(*false_block);
                        }
                    }
                    _ => {}
                }
            }
        }

        let orig_len = func.blocks.len();
        func.blocks.retain(|b| reachable.contains(&b.id));

        // Clean up phi incoming lists for removed blocks
        for block in &mut func.blocks {
            for phi in &mut block.phis {
                phi.incoming.retain(|(b, _)| reachable.contains(b));
            }
        }

        func.blocks.len() != orig_len
    }
}
