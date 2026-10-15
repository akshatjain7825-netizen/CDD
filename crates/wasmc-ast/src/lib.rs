use serde::{Deserialize, Serialize};
use wasmc_diagnostics::SourceSpan;
use wasmc_lexer::SymbolId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeIndex(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimType {
    Void,
    Bool,
    Char,
    I32,
    I64,
    U32,
    U64,
    F32,
    F64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralValue {
    Integer(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(SymbolId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,      // -
    Not,      // !
    BitNot,   // ~
    AddrOf,   // &
    Deref,    // *
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, Rem,
    Shl, Shr,
    BitAnd, BitOr, BitXor,
    Eq, Ne, Lt, Le, Gt, Ge,
    LogicAnd, LogicOr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructField {
    pub name: SymbolId,
    pub type_node: NodeIndex,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub name: SymbolId,
    pub type_node: NodeIndex,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AstNode {
    // Declarations
    ImportDecl {
        module: SymbolId,
        field: SymbolId,
        func_name: SymbolId,
        params: Vec<Param>,
        return_type: Option<NodeIndex>,
    },
    ExportDecl {
        decl: NodeIndex,
    },
    StructDecl {
        name: SymbolId,
        fields: Vec<StructField>,
    },
    GlobalDecl {
        name: SymbolId,
        mutable: bool,
        type_node: NodeIndex,
        init_expr: NodeIndex,
    },
    FunctionDecl {
        name: SymbolId,
        params: Vec<Param>,
        return_type: Option<NodeIndex>,
        body: NodeIndex,
    },

    // Types
    TypePrimitive(PrimType),
    TypePointer(NodeIndex),
    TypeArray(NodeIndex, u32),
    TypeFuncPtr {
        params: Vec<NodeIndex>,
        return_type: Option<NodeIndex>,
    },
    TypeStructRef(SymbolId),

    // Statements
    BlockStmt(Vec<NodeIndex>),
    LetStmt {
        name: SymbolId,
        mutable: bool,
        type_node: NodeIndex,
        init: Option<NodeIndex>,
    },
    AssignStmt {
        lhs: NodeIndex,
        rhs: NodeIndex,
    },
    IfStmt {
        cond: NodeIndex,
        then_block: NodeIndex,
        else_stmt: Option<NodeIndex>,
    },
    WhileStmt {
        cond: NodeIndex,
        body: NodeIndex,
    },
    ReturnStmt {
        is_tail: bool,
        value: Option<NodeIndex>,
    },
    BreakStmt,
    ContinueStmt,
    TryCatchStmt {
        try_block: NodeIndex,
        err_name: SymbolId,
        err_type: NodeIndex,
        catch_block: NodeIndex,
    },
    ThrowStmt(NodeIndex),
    ExprStmt(NodeIndex),
    ErrorStmt,

    // Expressions
    LiteralExpr(LiteralValue),
    VariableExpr(SymbolId),
    UnaryExpr {
        op: UnaryOp,
        operand: NodeIndex,
    },
    BinaryExpr {
        op: BinaryOp,
        lhs: NodeIndex,
        rhs: NodeIndex,
    },
    CallExpr {
        callee: NodeIndex,
        args: Vec<NodeIndex>,
    },
    IndexExpr {
        base: NodeIndex,
        index: NodeIndex,
    },
    MemberExpr {
        base: NodeIndex,
        member: SymbolId,
    },
    CastExpr {
        expr: NodeIndex,
        target_type: NodeIndex,
    },
    PoisonExpr,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AstPool {
    nodes: Vec<AstNode>,
    spans: Vec<SourceSpan>,
    pub top_level_decls: Vec<NodeIndex>,
}

impl AstPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc(&mut self, node: AstNode, span: SourceSpan) -> NodeIndex {
        let idx = NodeIndex(self.nodes.len() as u32);
        self.nodes.push(node);
        self.spans.push(span);
        idx
    }

    pub fn get(&self, idx: NodeIndex) -> &AstNode {
        &self.nodes[idx.0 as usize]
    }

    pub fn get_mut(&mut self, idx: NodeIndex) -> &mut AstNode {
        &mut self.nodes[idx.0 as usize]
    }

    pub fn span(&self, idx: NodeIndex) -> SourceSpan {
        self.spans[idx.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
