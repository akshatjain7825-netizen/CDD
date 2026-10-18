use wasmc_ast::*;
use wasmc_lexer::SymbolId;
use wasmc_semantics::{SemType, SemanticModel, StorageClass, SymbolEntry};

#[derive(Debug, Clone, PartialEq)]
pub enum HirExpr {
    Literal(LiteralValue, SemType),
    Variable(SymbolId, SemType, StorageClass),
    Unary(UnaryOp, Box<HirExpr>, SemType),
    Binary(BinaryOp, Box<HirExpr>, Box<HirExpr>, SemType),
    Call(SymbolId, Vec<HirExpr>, SemType),
    CallIndirect(Box<HirExpr>, Vec<HirExpr>, SemType),
    Load {
        ptr: Box<HirExpr>,
        offset: u32,
        align: u32,
        ty: SemType,
    },
    AddrOf(SymbolId, StorageClass, SemType),
    Cast(Box<HirExpr>, SemType),
}

impl HirExpr {
    pub fn ty(&self) -> &SemType {
        match self {
            HirExpr::Literal(_, t) => t,
            HirExpr::Variable(_, t, _) => t,
            HirExpr::Unary(_, _, t) => t,
            HirExpr::Binary(_, _, _, t) => t,
            HirExpr::Call(_, _, t) => t,
            HirExpr::CallIndirect(_, _, t) => t,
            HirExpr::Load { ty, .. } => ty,
            HirExpr::AddrOf(_, _, t) => t,
            HirExpr::Cast(_, t) => t,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirStmt {
    AssignVar(SymbolId, StorageClass, HirExpr),
    Store {
        ptr: HirExpr,
        val: HirExpr,
        offset: u32,
        align: u32,
    },
    If {
        cond: HirExpr,
        then_branch: Box<HirStmt>,
        else_branch: Option<Box<HirStmt>>,
    },
    While {
        cond: HirExpr,
        body: Box<HirStmt>,
    },
    Return {
        is_tail: bool,
        value: Option<HirExpr>,
    },
    Break,
    Continue,
    Block(Vec<HirStmt>),
    Expr(HirExpr),
    Throw(HirExpr),
    TryCatch {
        try_body: Box<HirStmt>,
        err_name: SymbolId,
        catch_body: Box<HirStmt>,
    },
}

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub name: SymbolId,
    pub params: Vec<SymbolEntry>,
    pub return_type: SemType,
    pub body: HirStmt,
    pub locals: Vec<SymbolEntry>,
    pub frame_size: u32,
}

#[derive(Debug, Clone)]
pub struct HirModule {
    pub functions: Vec<HirFunction>,
    pub globals: Vec<SymbolEntry>,
    pub model: SemanticModel,
}

pub struct HirLowerer<'a> {
    pool: &'a AstPool,
    model: &'a SemanticModel,
}

impl<'a> HirLowerer<'a> {
    pub fn new(pool: &'a AstPool, model: &'a SemanticModel) -> Self {
        Self { pool, model }
    }

    pub fn lower_module(self) -> HirModule {
        let mut functions = Vec::new();

        for &decl_idx in &self.pool.top_level_decls {
            let actual_idx = match self.pool.get(decl_idx) {
                AstNode::ExportDecl { decl } => *decl,
                _ => decl_idx,
            };

            if let AstNode::FunctionDecl {
                name,
                body,
                ..
            } = self.pool.get(actual_idx)
            {
                let fn_entry = &self.model.functions[name];
                let (params_sem, ret_sem) = match &fn_entry.sem_type {
                    SemType::Function { params, ret } => (params.clone(), *ret.clone()),
                    _ => (Vec::new(), SemType::Void),
                };

                let locals = self.model.func_locals.get(name).cloned().unwrap_or_default();
                let frame_size = self.model.func_frame_sizes.get(name).copied().unwrap_or(0);

                let params: Vec<SymbolEntry> = locals
                    .iter()
                    .take(params_sem.len())
                    .cloned()
                    .collect();

                let hir_body = self.lower_stmt(*body, &locals);

                functions.push(HirFunction {
                    name: *name,
                    params,
                    return_type: ret_sem,
                    body: hir_body,
                    locals,
                    frame_size,
                });
            }
        }

        let globals: Vec<SymbolEntry> = self.model.globals.values().cloned().collect();

        HirModule {
            functions,
            globals,
            model: self.model.clone(),
        }
    }

    fn lower_stmt(&self, idx: NodeIndex, locals: &[SymbolEntry]) -> HirStmt {
        let Some(node) = self.pool.try_get(idx) else {
            return HirStmt::Block(Vec::new());
        };
        match node {
            AstNode::BlockStmt(stmts) => {
                let lowered = stmts.iter().map(|&s| self.lower_stmt(s, locals)).collect();
                HirStmt::Block(lowered)
            }
            AstNode::LetStmt { name, init, .. } => {
                let storage = locals
                    .iter()
                    .find(|l| l.name == *name)
                    .map(|l| l.storage)
                    .unwrap_or(StorageClass::VirtualLocal { local_idx: 0 });

                if let Some(init_idx) = init {
                    let expr = self.lower_expr(*init_idx, locals);
                    match storage {
                        StorageClass::ShadowStack { align_bytes, .. } => {
                            let ptr = HirExpr::AddrOf(*name, storage, SemType::Pointer(Box::new(expr.ty().clone())));
                            HirStmt::Store {
                                ptr,
                                val: expr,
                                offset: 0,
                                align: align_bytes,
                            }
                        }
                        _ => HirStmt::AssignVar(*name, storage, expr),
                    }
                } else {
                    HirStmt::Block(Vec::new())
                }
            }
            AstNode::AssignStmt { lhs, rhs } => {
                let rhs_expr = self.lower_expr(*rhs, locals);
                match self.pool.get(*lhs) {
                    AstNode::VariableExpr(sym) => {
                        let storage = locals
                            .iter()
                            .find(|l| l.name == *sym)
                            .map(|l| l.storage)
                            .or_else(|| self.model.globals.get(sym).map(|g| g.storage))
                            .unwrap_or(StorageClass::VirtualLocal { local_idx: 0 });

                        match storage {
                            StorageClass::ShadowStack { align_bytes, .. } => {
                                let ptr = HirExpr::AddrOf(*sym, storage, SemType::Pointer(Box::new(rhs_expr.ty().clone())));
                                HirStmt::Store {
                                    ptr,
                                    val: rhs_expr,
                                    offset: 0,
                                    align: align_bytes,
                                }
                            }
                            _ => HirStmt::AssignVar(*sym, storage, rhs_expr),
                        }
                    }
                    AstNode::UnaryExpr { op: UnaryOp::Deref, operand } => {
                        let ptr_expr = self.lower_expr(*operand, locals);
                        let align = rhs_expr.ty().alignment();
                        HirStmt::Store {
                            ptr: ptr_expr,
                            val: rhs_expr,
                            offset: 0,
                            align,
                        }
                    }
                    _ => HirStmt::Expr(rhs_expr),
                }
            }
            AstNode::IfStmt {
                cond,
                then_block,
                else_stmt,
            } => HirStmt::If {
                cond: self.lower_expr(*cond, locals),
                then_branch: Box::new(self.lower_stmt(*then_block, locals)),
                else_branch: else_stmt.map(|e| Box::new(self.lower_stmt(e, locals))),
            },
            AstNode::WhileStmt { cond, body } => HirStmt::While {
                cond: self.lower_expr(*cond, locals),
                body: Box::new(self.lower_stmt(*body, locals)),
            },
            AstNode::ReturnStmt { is_tail, value } => HirStmt::Return {
                is_tail: *is_tail,
                value: value.map(|v| self.lower_expr(v, locals)),
            },
            AstNode::BreakStmt => HirStmt::Break,
            AstNode::ContinueStmt => HirStmt::Continue,
            AstNode::ThrowStmt(e) => HirStmt::Throw(self.lower_expr(*e, locals)),
            AstNode::TryCatchStmt {
                try_block,
                err_name,
                catch_block,
                ..
            } => HirStmt::TryCatch {
                try_body: Box::new(self.lower_stmt(*try_block, locals)),
                err_name: *err_name,
                catch_body: Box::new(self.lower_stmt(*catch_block, locals)),
            },
            AstNode::ExprStmt(e) => HirStmt::Expr(self.lower_expr(*e, locals)),
            _ => HirStmt::Block(Vec::new()),
        }
    }

    fn lower_expr(&self, idx: NodeIndex, locals: &[SymbolEntry]) -> HirExpr {
        let ty = self
            .model
            .node_types
            .get(&idx)
            .cloned()
            .unwrap_or(SemType::I32);

        let Some(node) = self.pool.try_get(idx) else {
            return HirExpr::Literal(LiteralValue::Integer(0), ty);
        };

        match node {
            AstNode::LiteralExpr(lit) => HirExpr::Literal(lit.clone(), ty),
            AstNode::VariableExpr(sym) => {
                let storage = locals
                    .iter()
                    .find(|l| l.name == *sym)
                    .map(|l| l.storage)
                    .or_else(|| self.model.globals.get(sym).map(|g| g.storage))
                    .unwrap_or(StorageClass::VirtualLocal { local_idx: 0 });

                match storage {
                    StorageClass::ShadowStack { frame_offset, align_bytes, .. } => {
                        let ptr = HirExpr::AddrOf(*sym, storage, SemType::Pointer(Box::new(ty.clone())));
                        HirExpr::Load {
                            ptr: Box::new(ptr),
                            offset: frame_offset as u32,
                            align: align_bytes,
                            ty,
                        }
                    }
                    _ => HirExpr::Variable(*sym, ty, storage),
                }
            }
            AstNode::UnaryExpr { op, operand } => match op {
                UnaryOp::AddrOf => {
                    if let AstNode::VariableExpr(sym) = self.pool.get(*operand) {
                        let storage = locals
                            .iter()
                            .find(|l| l.name == *sym)
                            .map(|l| l.storage)
                            .unwrap_or(StorageClass::VirtualLocal { local_idx: 0 });
                        HirExpr::AddrOf(*sym, storage, ty)
                    } else {
                        let inner = self.lower_expr(*operand, locals);
                        HirExpr::Unary(*op, Box::new(inner), ty)
                    }
                }
                UnaryOp::Deref => {
                    let ptr = self.lower_expr(*operand, locals);
                    let align = ty.alignment();
                    HirExpr::Load {
                        ptr: Box::new(ptr),
                        offset: 0,
                        align,
                        ty,
                    }
                }
                _ => {
                    let inner = self.lower_expr(*operand, locals);
                    HirExpr::Unary(*op, Box::new(inner), ty)
                }
            },
            AstNode::BinaryExpr { op, lhs, rhs } => {
                let l = self.lower_expr(*lhs, locals);
                let r = self.lower_expr(*rhs, locals);
                HirExpr::Binary(*op, Box::new(l), Box::new(r), ty)
            }
            AstNode::CallExpr { callee, args } => {
                let lowered_args = args.iter().map(|&a| self.lower_expr(a, locals)).collect();
                if let AstNode::VariableExpr(fn_name) = self.pool.get(*callee) {
                    HirExpr::Call(*fn_name, lowered_args, ty)
                } else {
                    let target = self.lower_expr(*callee, locals);
                    HirExpr::CallIndirect(Box::new(target), lowered_args, ty)
                }
            }
            AstNode::CastExpr { expr, .. } => {
                let inner = self.lower_expr(*expr, locals);
                HirExpr::Cast(Box::new(inner), ty)
            }
            _ => HirExpr::Literal(LiteralValue::Integer(0), SemType::I32),
        }
    }
}
