use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use wasmc_ast::*;
use wasmc_diagnostics::{Diagnostic, DiagnosticSink};
use wasmc_lexer::SymbolId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SemType {
    Void,
    Bool,
    Char,
    I32,
    I64,
    U32,
    U64,
    F32,
    F64,
    Pointer(Box<SemType>),
    Array(Box<SemType>, u32),
    Function {
        params: Vec<SemType>,
        ret: Box<SemType>,
    },
    Struct(SymbolId),
    Poison,
}

impl SemType {
    pub fn is_integer(&self) -> bool {
        matches!(self, SemType::I32 | SemType::I64 | SemType::U32 | SemType::U64)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, SemType::F32 | SemType::F64)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    pub fn size_in_bytes(&self) -> u32 {
        match self {
            SemType::Void => 0,
            SemType::Bool | SemType::Char => 1,
            SemType::I32 | SemType::U32 | SemType::F32 | SemType::Pointer(_) => 4,
            SemType::I64 | SemType::U64 | SemType::F64 => 8,
            SemType::Array(inner, len) => inner.size_in_bytes() * len,
            SemType::Function { .. } => 4, // table index
            SemType::Struct(_) => 16,     // default alignment/minimum
            SemType::Poison => 0,
        }
    }

    pub fn alignment(&self) -> u32 {
        match self {
            SemType::Void => 1,
            SemType::Bool | SemType::Char => 1,
            SemType::I32 | SemType::U32 | SemType::F32 | SemType::Pointer(_) => 4,
            SemType::I64 | SemType::U64 | SemType::F64 => 8,
            SemType::Array(inner, _) => inner.alignment(),
            SemType::Function { .. } => 4,
            SemType::Struct(_) => 8,
            SemType::Poison => 1,
        }
    }

    pub fn unifies_with(&self, other: &SemType) -> bool {
        if *self == SemType::Poison || *other == SemType::Poison {
            return true;
        }
        self == other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageClass {
    VirtualLocal { local_idx: u32 },
    ShadowStack { frame_offset: i32, size_bytes: u32, align_bytes: u32 },
    GlobalSlot { global_idx: u32, mutable: bool },
    FunctionRef { func_idx: u32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolEntry {
    pub name: SymbolId,
    pub sem_type: SemType,
    pub storage: StorageClass,
    pub is_address_taken: bool,
    pub is_mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructDef {
    pub name: SymbolId,
    pub fields: Vec<(SymbolId, SemType)>,
    pub size_bytes: u32,
    pub align_bytes: u32,
}

#[derive(Debug, Default, Clone)]
pub struct Scope {
    symbols: HashMap<SymbolId, SymbolEntry>,
}

#[derive(Debug, Clone)]
pub struct SemanticModel {
    pub node_types: HashMap<NodeIndex, SemType>,
    pub struct_defs: HashMap<SymbolId, StructDef>,
    pub functions: HashMap<SymbolId, SymbolEntry>,
    pub globals: HashMap<SymbolId, SymbolEntry>,
    pub func_locals: HashMap<SymbolId, Vec<SymbolEntry>>,
    pub func_frame_sizes: HashMap<SymbolId, u32>,
}

pub struct SemanticAnalyzer<'a> {
    pool: &'a AstPool,
    sink: &'a mut DiagnosticSink,
    scopes: Vec<Scope>,
    model: SemanticModel,
    current_func: Option<SymbolId>,
    virtual_local_counter: u32,
    func_counter: u32,
    global_counter: u32,
}

impl<'a> SemanticAnalyzer<'a> {
    pub fn new(pool: &'a AstPool, sink: &'a mut DiagnosticSink) -> Self {
        Self {
            pool,
            sink,
            scopes: vec![Scope::default()],
            model: SemanticModel {
                node_types: HashMap::new(),
                struct_defs: HashMap::new(),
                functions: HashMap::new(),
                globals: HashMap::new(),
                func_locals: HashMap::new(),
                func_frame_sizes: HashMap::new(),
            },
            current_func: None,
            virtual_local_counter: 0,
            func_counter: 0,
            global_counter: 0,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn insert_symbol(&mut self, entry: SymbolEntry) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.symbols.insert(entry.name, entry);
        }
    }

    fn lookup_symbol(&self, name: SymbolId) -> Option<&SymbolEntry> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.symbols.get(&name) {
                return Some(entry);
            }
        }
        self.model.globals.get(&name).or_else(|| self.model.functions.get(&name))
    }

    fn lookup_symbol_mut(&mut self, name: SymbolId) -> Option<&mut SymbolEntry> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(entry) = scope.symbols.get_mut(&name) {
                return Some(entry);
            }
        }
        if let Some(entry) = self.model.globals.get_mut(&name) {
            return Some(entry);
        }
        self.model.functions.get_mut(&name)
    }

    pub fn analyze(mut self) -> Result<SemanticModel, DiagnosticSink> {
        // Pass 1: Collect structs, globals, imports, and function signatures
        for &decl_idx in &self.pool.top_level_decls {
            self.collect_decl(decl_idx);
        }

        // Pass 2: Typecheck function bodies and perform address-taken escape analysis
        for &decl_idx in &self.pool.top_level_decls {
            self.check_decl(decl_idx);
        }

        if self.sink.has_errors() {
            Err(self.sink.clone())
        } else {
            Ok(self.model)
        }
    }

    fn resolve_type_node(&self, idx: NodeIndex) -> SemType {
        match self.pool.get(idx) {
            AstNode::TypePrimitive(p) => match p {
                PrimType::Void => SemType::Void,
                PrimType::Bool => SemType::Bool,
                PrimType::Char => SemType::Char,
                PrimType::I32 => SemType::I32,
                PrimType::I64 => SemType::I64,
                PrimType::U32 => SemType::U32,
                PrimType::U64 => SemType::U64,
                PrimType::F32 => SemType::F32,
                PrimType::F64 => SemType::F64,
            },
            AstNode::TypePointer(inner) => SemType::Pointer(Box::new(self.resolve_type_node(*inner))),
            AstNode::TypeArray(inner, len) => SemType::Array(Box::new(self.resolve_type_node(*inner)), *len),
            AstNode::TypeFuncPtr { params, return_type } => {
                let p_types = params.iter().map(|&p| self.resolve_type_node(p)).collect();
                let ret = return_type
                    .map(|r| Box::new(self.resolve_type_node(r)))
                    .unwrap_or_else(|| Box::new(SemType::Void));
                SemType::Function { params: p_types, ret }
            }
            AstNode::TypeStructRef(sym) => SemType::Struct(*sym),
            _ => SemType::Poison,
        }
    }

    fn collect_decl(&mut self, idx: NodeIndex) {
        match self.pool.get(idx) {
            AstNode::ExportDecl { decl } => self.collect_decl(*decl),
            AstNode::StructDecl { name, fields } => {
                let mut def_fields = Vec::new();
                let mut offset = 0;
                let mut max_align = 4;
                for f in fields {
                    let sem = self.resolve_type_node(f.type_node);
                    let align = sem.alignment();
                    max_align = max_align.max(align);
                    offset = (offset + align - 1) & !(align - 1);
                    offset += sem.size_in_bytes();
                    def_fields.push((f.name, sem));
                }
                let total_size = ((offset + max_align - 1) & !(max_align - 1)).max(max_align);
                self.model.struct_defs.insert(
                    *name,
                    StructDef {
                        name: *name,
                        fields: def_fields,
                        size_bytes: total_size,
                        align_bytes: max_align,
                    },
                );
            }
            AstNode::GlobalDecl {
                name,
                mutable,
                type_node,
                ..
            } => {
                let ty = self.resolve_type_node(*type_node);
                let entry = SymbolEntry {
                    name: *name,
                    sem_type: ty,
                    storage: StorageClass::GlobalSlot {
                        global_idx: self.global_counter,
                        mutable: *mutable,
                    },
                    is_address_taken: false,
                    is_mutable: *mutable,
                };
                self.global_counter += 1;
                self.model.globals.insert(*name, entry);
            }
            AstNode::ImportDecl {
                func_name,
                params,
                return_type,
                ..
            } => {
                let p_types = params.iter().map(|p| self.resolve_type_node(p.type_node)).collect();
                let ret = return_type
                    .map(|r| Box::new(self.resolve_type_node(r)))
                    .unwrap_or_else(|| Box::new(SemType::Void));
                let ty = SemType::Function { params: p_types, ret };
                let entry = SymbolEntry {
                    name: *func_name,
                    sem_type: ty,
                    storage: StorageClass::FunctionRef {
                        func_idx: self.func_counter,
                    },
                    is_address_taken: false,
                    is_mutable: false,
                };
                self.func_counter += 1;
                self.model.functions.insert(*func_name, entry);
            }
            AstNode::FunctionDecl {
                name,
                params,
                return_type,
                ..
            } => {
                if self.model.functions.contains_key(name) || self.model.globals.contains_key(name) {
                    self.sink.push(Diagnostic::error(
                        "E3001",
                        format!("Duplicate symbol definition: symbol ID {:?}", name),
                        self.pool.span(idx),
                    ));
                    return;
                }
                let p_types = params.iter().map(|p| self.resolve_type_node(p.type_node)).collect();
                let ret = return_type
                    .map(|r| Box::new(self.resolve_type_node(r)))
                    .unwrap_or_else(|| Box::new(SemType::Void));
                let ty = SemType::Function { params: p_types, ret };
                let entry = SymbolEntry {
                    name: *name,
                    sem_type: ty,
                    storage: StorageClass::FunctionRef {
                        func_idx: self.func_counter,
                    },
                    is_address_taken: false,
                    is_mutable: false,
                };
                self.func_counter += 1;
                self.model.functions.insert(*name, entry);
            }
            _ => {}
        }
    }

    fn check_decl(&mut self, idx: NodeIndex) {
        match self.pool.get(idx) {
            AstNode::ExportDecl { decl } => self.check_decl(*decl),
            AstNode::GlobalDecl {
                type_node,
                init_expr,
                ..
            } => {
                let expected = self.resolve_type_node(*type_node);
                let actual = self.check_expr(*init_expr, &expected, &mut []);
                if !expected.unifies_with(&actual) {
                    self.sink.push(Diagnostic::error(
                        "E3002",
                        format!("Type mismatch in global initializer: expected {:?}, got {:?}", expected, actual),
                        self.pool.span(*init_expr),
                    ));
                }
            }
            AstNode::FunctionDecl {
                name,
                params,
                return_type,
                body,
            } => {
                self.current_func = Some(*name);
                self.virtual_local_counter = 0;
                self.push_scope();

                let mut locals = Vec::new();

                // Register parameters as virtual locals
                for p in params {
                    let ty = self.resolve_type_node(p.type_node);
                    let entry = SymbolEntry {
                        name: p.name,
                        sem_type: ty,
                        storage: StorageClass::VirtualLocal {
                            local_idx: self.virtual_local_counter,
                        },
                        is_address_taken: false,
                        is_mutable: false,
                    };
                    self.virtual_local_counter += 1;
                    self.insert_symbol(entry.clone());
                    locals.push(entry);
                }

                let ret_ty = return_type
                    .map(|r| self.resolve_type_node(r))
                    .unwrap_or(SemType::Void);

                // Check function body
                self.check_stmt(*body, &ret_ty, &mut locals);

                // Escape analysis & shadow-stack frame layout (Section 4.7.1)
                let mut frame_offset = 0i32;
                for loc in &mut locals {
                    if loc.is_address_taken || matches!(loc.sem_type, SemType::Struct(_) | SemType::Array(_, _)) {
                        let size = loc.sem_type.size_in_bytes();
                        let align = loc.sem_type.alignment() as i32;
                        frame_offset = (frame_offset + align - 1) & !(align - 1);
                        loc.storage = StorageClass::ShadowStack {
                            frame_offset,
                            size_bytes: size,
                            align_bytes: align as u32,
                        };
                        frame_offset += size as i32;
                    }
                }
                let aligned_frame_size = ((frame_offset + 15) / 16 * 16) as u32;

                self.model.func_frame_sizes.insert(*name, aligned_frame_size);
                self.model.func_locals.insert(*name, locals);

                self.pop_scope();
                self.current_func = None;
            }
            _ => {}
        }
    }

    fn check_stmt(&mut self, idx: NodeIndex, expected_ret: &SemType, locals: &mut Vec<SymbolEntry>) {
        match self.pool.get(idx) {
            AstNode::BlockStmt(stmts) => {
                self.push_scope();
                for &s in stmts {
                    self.check_stmt(s, expected_ret, locals);
                }
                self.pop_scope();
            }
            AstNode::LetStmt {
                name,
                mutable,
                type_node,
                init,
            } => {
                let ty = self.resolve_type_node(*type_node);
                if let Some(init_idx) = init {
                    let actual = self.check_expr(*init_idx, &ty, locals);
                    if !ty.unifies_with(&actual) {
                        self.sink.push(Diagnostic::error(
                            "E3002",
                            format!("Type mismatch in variable initialization: expected {:?}, got {:?}", ty, actual),
                            self.pool.span(*init_idx),
                        ));
                    }
                }
                let entry = SymbolEntry {
                    name: *name,
                    sem_type: ty,
                    storage: StorageClass::VirtualLocal {
                        local_idx: self.virtual_local_counter,
                    },
                    is_address_taken: false,
                    is_mutable: *mutable,
                };
                self.virtual_local_counter += 1;
                self.insert_symbol(entry.clone());
                locals.push(entry);
            }
            AstNode::AssignStmt { lhs, rhs } => {
                let lhs_ty = self.synthesize_expr(*lhs, locals);
                let rhs_ty = self.check_expr(*rhs, &lhs_ty, locals);
                if !lhs_ty.unifies_with(&rhs_ty) {
                    self.sink.push(Diagnostic::error(
                        "E3002",
                        format!("Cannot assign {:?} to variable of type {:?}", rhs_ty, lhs_ty),
                        self.pool.span(*rhs),
                    ));
                }
            }
            AstNode::IfStmt {
                cond,
                then_block,
                else_stmt,
            } => {
                let cond_ty = self.check_expr(*cond, &SemType::Bool, locals);
                if !cond_ty.unifies_with(&SemType::Bool) && !cond_ty.unifies_with(&SemType::I32) {
                    self.sink.push(Diagnostic::error(
                        "E3003",
                        format!("Condition must evaluate to bool or i32, got {:?}", cond_ty),
                        self.pool.span(*cond),
                    ));
                }
                self.check_stmt(*then_block, expected_ret, locals);
                if let Some(else_idx) = else_stmt {
                    self.check_stmt(*else_idx, expected_ret, locals);
                }
            }
            AstNode::WhileStmt { cond, body } => {
                let cond_ty = self.check_expr(*cond, &SemType::Bool, locals);
                if !cond_ty.unifies_with(&SemType::Bool) && !cond_ty.unifies_with(&SemType::I32) {
                    self.sink.push(Diagnostic::error(
                        "E3003",
                        format!("Condition must evaluate to bool or i32, got {:?}", cond_ty),
                        self.pool.span(*cond),
                    ));
                }
                self.check_stmt(*body, expected_ret, locals);
            }
            AstNode::ReturnStmt { value, .. } => {
                if let Some(val_idx) = value {
                    let actual = self.check_expr(*val_idx, expected_ret, locals);
                    if !expected_ret.unifies_with(&actual) {
                        self.sink.push(Diagnostic::error(
                            "E3004",
                            format!("Return type mismatch: expected {:?}, got {:?}", expected_ret, actual),
                            self.pool.span(*val_idx),
                        ));
                    }
                } else if *expected_ret != SemType::Void && *expected_ret != SemType::Poison {
                    self.sink.push(Diagnostic::error(
                        "E3004",
                        format!("Missing return value for non-void function returning {:?}", expected_ret),
                        self.pool.span(idx),
                    ));
                }
            }
            AstNode::TryCatchStmt {
                try_block,
                err_name,
                err_type,
                catch_block,
            } => {
                self.check_stmt(*try_block, expected_ret, locals);
                self.push_scope();
                let e_type = self.resolve_type_node(*err_type);
                let entry = SymbolEntry {
                    name: *err_name,
                    sem_type: e_type,
                    storage: StorageClass::VirtualLocal {
                        local_idx: self.virtual_local_counter,
                    },
                    is_address_taken: false,
                    is_mutable: false,
                };
                self.virtual_local_counter += 1;
                self.insert_symbol(entry.clone());
                locals.push(entry);
                self.check_stmt(*catch_block, expected_ret, locals);
                self.pop_scope();
            }
            AstNode::ThrowStmt(expr) => {
                self.synthesize_expr(*expr, locals);
            }
            AstNode::ExprStmt(expr) => {
                self.synthesize_expr(*expr, locals);
            }
            AstNode::BreakStmt | AstNode::ContinueStmt | AstNode::ErrorStmt => {}
            _ => {}
        }
    }

    fn synthesize_expr(&mut self, idx: NodeIndex, locals: &mut [SymbolEntry]) -> SemType {
        let ty = match self.pool.get(idx) {
            AstNode::LiteralExpr(lit) => match lit {
                LiteralValue::Integer(_) => SemType::I32,
                LiteralValue::Float(_) => SemType::F64,
                LiteralValue::Bool(_) => SemType::Bool,
                LiteralValue::Char(_) => SemType::Char,
                LiteralValue::String(_) => SemType::Pointer(Box::new(SemType::Char)),
            },
            AstNode::VariableExpr(sym) => {
                if let Some(entry) = self.lookup_symbol(*sym) {
                    entry.sem_type.clone()
                } else {
                    self.sink.push(Diagnostic::error(
                        "E3003",
                        format!("Unresolved identifier symbol: {:?}", sym),
                        self.pool.span(idx),
                    ));
                    SemType::Poison
                }
            }
            AstNode::UnaryExpr { op, operand } => {
                let opnd_ty = self.synthesize_expr(*operand, locals);
                match op {
                    UnaryOp::Neg => {
                        if !opnd_ty.is_numeric() && opnd_ty != SemType::Poison {
                            self.sink.push(Diagnostic::error(
                                "E3005",
                                format!("Unary negation requires numeric type, got {:?}", opnd_ty),
                                self.pool.span(idx),
                            ));
                            SemType::Poison
                        } else {
                            opnd_ty
                        }
                    }
                    UnaryOp::Not => {
                        if opnd_ty != SemType::Bool && opnd_ty != SemType::I32 && opnd_ty != SemType::Poison {
                            self.sink.push(Diagnostic::error(
                                "E3005",
                                format!("Logical NOT requires bool or i32, got {:?}", opnd_ty),
                                self.pool.span(idx),
                            ));
                        }
                        SemType::Bool
                    }
                    UnaryOp::BitNot => opnd_ty,
                    UnaryOp::AddrOf => {
                        // Mark target variable as escaped to linear memory
                        if let AstNode::VariableExpr(sym) = self.pool.get(*operand) {
                            if let Some(entry) = self.lookup_symbol_mut(*sym) {
                                entry.is_address_taken = true;
                            }
                            for loc in locals.iter_mut() {
                                if loc.name == *sym {
                                    loc.is_address_taken = true;
                                }
                            }
                        }
                        SemType::Pointer(Box::new(opnd_ty))
                    }
                    UnaryOp::Deref => {
                        if let SemType::Pointer(inner) = opnd_ty {
                            *inner
                        } else if opnd_ty == SemType::Poison {
                            SemType::Poison
                        } else {
                            self.sink.push(Diagnostic::error(
                                "E3006",
                                format!("Cannot dereference non-pointer type {:?}", opnd_ty),
                                self.pool.span(idx),
                            ));
                            SemType::Poison
                        }
                    }
                }
            }
            AstNode::BinaryExpr { op, lhs, rhs } => {
                let l_ty = self.synthesize_expr(*lhs, locals);
                let r_ty = self.synthesize_expr(*rhs, locals);

                if !l_ty.unifies_with(&r_ty) {
                    self.sink.push(Diagnostic::error(
                        "E3007",
                        format!("Binary operator {:?} operands type mismatch: {:?} vs {:?}", op, l_ty, r_ty),
                        self.pool.span(idx),
                    ));
                    SemType::Poison
                } else {
                    match op {
                        BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Rem
                        | BinaryOp::Shl
                        | BinaryOp::Shr
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor => l_ty,
                        BinaryOp::Eq
                        | BinaryOp::Ne
                        | BinaryOp::Lt
                        | BinaryOp::Le
                        | BinaryOp::Gt
                        | BinaryOp::Ge
                        | BinaryOp::LogicAnd
                        | BinaryOp::LogicOr => SemType::Bool,
                    }
                }
            }
            AstNode::CallExpr { callee, args } => {
                let callee_ty = self.synthesize_expr(*callee, locals);
                match callee_ty {
                    SemType::Function { params, ret } => {
                        if args.len() != params.len() {
                            self.sink.push(Diagnostic::error(
                                "E3008",
                                format!("Call argument count mismatch: expected {}, got {}", params.len(), args.len()),
                                self.pool.span(idx),
                            ));
                        } else {
                            for (a, p) in args.iter().zip(params.iter()) {
                                self.check_expr(*a, p, locals);
                            }
                        }
                        *ret
                    }
                    SemType::Poison => SemType::Poison,
                    _ => {
                        self.sink.push(Diagnostic::error(
                            "E3009",
                            format!("Cannot invoke non-function type {:?}", callee_ty),
                            self.pool.span(idx),
                        ));
                        SemType::Poison
                    }
                }
            }
            AstNode::IndexExpr { base, index } => {
                let base_ty = self.synthesize_expr(*base, locals);
                let idx_ty = self.check_expr(*index, &SemType::I32, locals);
                if !idx_ty.is_integer() && idx_ty != SemType::Poison {
                    self.sink.push(Diagnostic::error(
                        "E3010",
                        format!("Array index must be integer, got {:?}", idx_ty),
                        self.pool.span(*index),
                    ));
                }
                match base_ty {
                    SemType::Array(inner, _) => *inner,
                    SemType::Pointer(inner) => *inner,
                    SemType::Poison => SemType::Poison,
                    _ => {
                        self.sink.push(Diagnostic::error(
                            "E3011",
                            format!("Cannot index non-array/non-pointer type {:?}", base_ty),
                            self.pool.span(idx),
                        ));
                        SemType::Poison
                    }
                }
            }
            AstNode::MemberExpr { base, member } => {
                let base_ty = self.synthesize_expr(*base, locals);
                match base_ty {
                    SemType::Struct(s_name) => {
                        if let Some(s_def) = self.model.struct_defs.get(&s_name) {
                            if let Some((_, f_ty)) = s_def.fields.iter().find(|(n, _)| n == member) {
                                f_ty.clone()
                            } else {
                                self.sink.push(Diagnostic::error(
                                    "E3012",
                                    format!("Struct has no field with identifier {:?}", member),
                                    self.pool.span(idx),
                                ));
                                SemType::Poison
                            }
                        } else {
                            SemType::Poison
                        }
                    }
                    SemType::Poison => SemType::Poison,
                    _ => {
                        self.sink.push(Diagnostic::error(
                            "E3013",
                            format!("Member access on non-struct type {:?}", base_ty),
                            self.pool.span(idx),
                        ));
                        SemType::Poison
                    }
                }
            }
            AstNode::CastExpr { expr, target_type } => {
                self.synthesize_expr(*expr, locals);
                self.resolve_type_node(*target_type)
            }
            AstNode::PoisonExpr => SemType::Poison,
            _ => SemType::Poison,
        };

        self.model.node_types.insert(idx, ty.clone());
        ty
    }

    fn check_expr(&mut self, idx: NodeIndex, expected: &SemType, locals: &mut [SymbolEntry]) -> SemType {
        let synthesized = self.synthesize_expr(idx, locals);
        if !expected.unifies_with(&synthesized) {
            self.sink.push(Diagnostic::error(
                "E3002",
                format!("Expected type {:?}, got {:?}", expected, synthesized),
                self.pool.span(idx),
            ));
        }
        synthesized
    }
}
