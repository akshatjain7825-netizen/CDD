use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use wasmc_lexer::SymbolId;
use wasmc_mir::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructuredScopeKind {
    Block,
    Loop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StructuredInst {
    BlockStart,
    LoopStart,
    End,
    Br(u32),
    BrIf(u32, VReg),
    BrIfEqz(u32, VReg),
    Mir(MirInst),
    Return(Option<VReg>),
    TailCall {
        func_name: SymbolId,
        args: Vec<VReg>,
    },
    Throw(VReg),
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredFunction {
    pub name: SymbolId,
    pub params: Vec<(VReg, MirType)>,
    pub return_type: MirType,
    pub instructions: Vec<StructuredInst>,
    pub vreg_types: HashMap<VReg, MirType>,
    pub frame_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredModule {
    pub functions: Vec<StructuredFunction>,
    pub globals: Vec<wasmc_semantics::SymbolEntry>,
}

pub struct Stackifier;

impl Stackifier {
    pub fn stackify_module(module: MirModule) -> StructuredModule {
        let mut functions = Vec::new();
        for f in module.functions {
            functions.push(Self::stackify_function(f));
        }
        StructuredModule {
            functions,
            globals: module.globals,
        }
    }

    pub fn stackify_function(mut func: MirFunction) -> StructuredFunction {
        // Step 1: Deconstruct Phi nodes into predecessor parallel copies
        Self::deconstruct_phis(&mut func);

        // Step 2: Compute Reverse Post-Order (RPO)
        let rpo = Self::compute_rpo(&func);
        let rpo_indices: HashMap<BlockId, usize> = rpo.iter().enumerate().map(|(idx, &b)| (b, idx)).collect();

        // Step 3: Classify back-edges (loop headers) and forward branch targets
        let mut loop_headers: HashSet<BlockId> = HashSet::new();
        let mut loop_last_back_edge: HashMap<BlockId, BlockId> = HashMap::new();

        for &b_id in &rpo {
            let Some(block) = func.blocks.iter().find(|b| b.id == b_id) else {
                continue;
            };
            let src_idx = rpo_indices[&b_id];

            for succ in &block.successors {
                if let Some(&dst_idx) = rpo_indices.get(succ) {
                    if dst_idx <= src_idx {
                        // Backward edge -> succ is a loop header
                        loop_headers.insert(*succ);
                        loop_last_back_edge.insert(*succ, b_id);
                    }
                }
            }
        }

        // Step 4: Control Stack for relative branch depth calculation
        let mut out = Vec::new();
        let mut context_stack: Vec<(StructuredScopeKind, BlockId)> = Vec::new();

        for (i, &b_id) in rpo.iter().enumerate() {
            let Some(block) = func.blocks.iter().find(|b| b.id == b_id) else {
                continue;
            };

            // Close any Block scopes whose target is b_id
            while let Some(&(StructuredScopeKind::Block, target)) = context_stack.last() {
                if target == b_id {
                    context_stack.pop();
                    out.push(StructuredInst::End);
                } else {
                    break;
                }
            }

            // If b_id is a loop header, open a Loop scope
            if loop_headers.contains(&b_id) {
                context_stack.push((StructuredScopeKind::Loop, b_id));
                out.push(StructuredInst::LoopStart);
            }

            // Open Block scopes for forward edges originating at or after b_id
            let mut forward_targets: Vec<BlockId> = block
                .successors
                .iter()
                .filter(|&&succ| {
                    let dst_idx = rpo_indices[&succ];
                    dst_idx > i && !context_stack.iter().any(|&(_, t)| t == succ)
                })
                .copied()
                .collect();

            // Outermost target first (highest RPO index)
            forward_targets.sort_by_key(|&t| std::cmp::Reverse(rpo_indices[&t]));
            for target in forward_targets {
                context_stack.push((StructuredScopeKind::Block, target));
                out.push(StructuredInst::BlockStart);
            }

            // Emit basic block instructions
            for inst in &block.instructions {
                out.push(StructuredInst::Mir(inst.clone()));
            }

            // Emit terminator using relative branch depth lookup
            match &block.terminator {
                Terminator::Jump(target) => {
                    let next_in_rpo = rpo.get(i + 1).copied();
                    if next_in_rpo != Some(*target) {
                        let depth = Self::find_depth(&context_stack, *target);
                        out.push(StructuredInst::Br(depth));
                    }
                }
                Terminator::Branch { cond, true_block, false_block } => {
                    let depth_true = Self::find_depth(&context_stack, *true_block);
                    let depth_false = Self::find_depth(&context_stack, *false_block);

                    let next_in_rpo = rpo.get(i + 1).copied();
                    if next_in_rpo == Some(*false_block) {
                        out.push(StructuredInst::BrIf(depth_true, *cond));
                    } else if next_in_rpo == Some(*true_block) {
                        // Invert condition by testing eqz: branch to false_block when cond == 0
                        out.push(StructuredInst::BrIfEqz(depth_false, *cond));
                    } else {
                        out.push(StructuredInst::BrIf(depth_true, *cond));
                        out.push(StructuredInst::Br(depth_false));
                    }
                }
                Terminator::Return(v) => {
                    out.push(StructuredInst::Return(*v));
                }
                Terminator::TailCall { func_name, args } => {
                    out.push(StructuredInst::TailCall {
                        func_name: *func_name,
                        args: args.clone(),
                    });
                }
                Terminator::Throw(v) => {
                    out.push(StructuredInst::Throw(*v));
                }
                Terminator::Unreachable => {
                    out.push(StructuredInst::Unreachable);
                }
            }

            // Close Loop scopes whose furthest back-edge was b_id
            while let Some(&(StructuredScopeKind::Loop, header)) = context_stack.last() {
                if loop_last_back_edge.get(&header) == Some(&b_id) {
                    context_stack.pop();
                    out.push(StructuredInst::End);
                } else {
                    break;
                }
            }
        }

        // Close any remaining scopes
        while context_stack.pop().is_some() {
            out.push(StructuredInst::End);
        }

        StructuredFunction {
            name: func.name,
            params: func.params,
            return_type: func.return_type,
            instructions: out,
            vreg_types: func.vreg_types,
            frame_size: func.frame_size,
        }
    }

    fn find_depth(context_stack: &[(StructuredScopeKind, BlockId)], target: BlockId) -> u32 {
        for (depth, (_, t)) in context_stack.iter().rev().enumerate() {
            if *t == target {
                return depth as u32;
            }
        }
        0
    }



    fn sequentialize_copies(
        copies: Vec<(VReg, VReg, MirType)>,
        next_vreg: &mut u32,
        vreg_types: &mut HashMap<VReg, MirType>,
    ) -> Vec<MirInst> {
        let mut pending: Vec<(VReg, VReg, MirType)> =
            copies.into_iter().filter(|(d, s, _)| d != s).collect();
        let mut insts = Vec::new();

        while !pending.is_empty() {
            let mut ready_idx = None;
            for (i, (d, _, _)) in pending.iter().enumerate() {
                let is_src_of_another = pending
                    .iter()
                    .enumerate()
                    .any(|(j, (_, s, _))| j != i && s == d);
                if !is_src_of_another {
                    ready_idx = Some(i);
                    break;
                }
            }

            if let Some(idx) = ready_idx {
                let (dest, src, _) = pending.remove(idx);
                insts.push(MirInst::Copy { dest, src });
            } else {
                let (_, first_src, ty) = pending[0];
                let tmp = VReg(*next_vreg);
                *next_vreg += 1;
                vreg_types.insert(tmp, ty);

                insts.push(MirInst::Copy {
                    dest: tmp,
                    src: first_src,
                });

                for (_, s, _) in &mut pending {
                    if *s == first_src {
                        *s = tmp;
                    }
                }
            }
        }

        insts
    }

    fn deconstruct_phis(func: &mut MirFunction) {
        // Critical edges in wasmc structured CFGs are handled by parallel copy sequentialization
        // without synthetic landing pads that disrupt RPO structured block nesting.

        let block_phis: Vec<(BlockId, Vec<PhiNode>)> = func
            .blocks
            .iter_mut()
            .map(|b| (b.id, std::mem::take(&mut b.phis)))
            .collect();

        // Group incoming copies by predecessor block
        let mut pred_copies: HashMap<BlockId, Vec<(VReg, VReg, MirType)>> = HashMap::new();
        for (_phi_block, phis) in block_phis {
            for phi in phis {
                for (pred_block, src_vreg) in phi.incoming {
                    pred_copies
                        .entry(pred_block)
                        .or_default()
                        .push((phi.dest, src_vreg, phi.ty));
                }
            }
        }

        let mut next_vreg = func.vreg_types.keys().map(|v| v.0).max().unwrap_or(0) + 1;

        for (pred_block, copies) in pred_copies {
            let insts = Self::sequentialize_copies(copies, &mut next_vreg, &mut func.vreg_types);
            if let Some(pred) = func.blocks.iter_mut().find(|b| b.id == pred_block) {
                pred.instructions.extend(insts);
            }
        }
    }

    fn compute_rpo(func: &MirFunction) -> Vec<BlockId> {
        let mut visited = HashSet::new();
        let mut post_order = Vec::new();

        let block_map: HashMap<BlockId, &BasicBlock> = func.blocks.iter().map(|b| (b.id, b)).collect();

        fn dfs(
            b_id: BlockId,
            block_map: &HashMap<BlockId, &BasicBlock>,
            visited: &mut HashSet<BlockId>,
            post_order: &mut Vec<BlockId>,
        ) {
            visited.insert(b_id);
            if let Some(block) = block_map.get(&b_id) {
                // Visit non-terminal successors first, terminal (Return/Throw) last
                // so terminal blocks end up immediately adjacent to their predecessor in RPO.
                let mut succs = block.successors.clone();
                succs.sort_by_key(|succ| {
                    if let Some(sb) = block_map.get(succ) {
                        matches!(sb.terminator, Terminator::Return(_) | Terminator::Throw(_))
                    } else {
                        false
                    }
                });

                for succ in succs {
                    if !visited.contains(&succ) && block_map.contains_key(&succ) {
                        dfs(succ, block_map, visited, post_order);
                    }
                }
            }
            post_order.push(b_id);
        }

        dfs(func.entry_block, &block_map, &mut visited, &mut post_order);

        // Include any remaining reachable blocks
        for block in &func.blocks {
            if !visited.contains(&block.id) {
                dfs(block.id, &block_map, &mut visited, &mut post_order);
            }
        }

        post_order.reverse();
        post_order
    }
}
