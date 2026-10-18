# Low-Level Design (LLD): Direct-to-WebAssembly Compiler (`wasmc`)

**Document ID:** LLD-WASMC-2026-001

**Version:** 1.0.0

**Date:** October 14, 2026

**Author:** Senior Software Architect

**Parent Document:** `HLD.md` (`HLD-WASMC-2026-001`)

**Source Documents:** `wasm_compiler_srs.md` (v1.0.0), `wasm_compiler_frd.md` (v1.0.0)

## Table of Contents

 1. [Document Overview and HLD Reference](#1-document-overview-and-hld-reference)

 2. [Detailed Module and Component Design](#2-detailed-module-and-component-design)

 3. [API and CLI Specifications](#3-api-and-cli-specifications)

 4. [Database Design (Incremental Compilation Cache)](#4-database-design-incremental-compilation-cache)

 5. [Sequence Diagrams for Key Functional Flows](#5-sequence-diagrams-for-key-functional-flows)

 6. [Class, Struct, and Interface Definitions](#6-class-struct-and-interface-definitions)

 7. [Core Algorithms and Business Logic](#7-core-algorithms-and-business-logic)

 8. [Error Handling, Diagnostics, and Logging Strategy](#8-error-handling-diagnostics-and-logging-strategy)

 9. [Security Implementation Details](#9-security-implementation-details)

10. [Testing Considerations and Scenario Mapping](#10-testing-considerations-and-scenario-mapping)

11. [Open Questions](#11-open-questions)

12. [LLD Requirement Traceability Matrix](#12-lld-requirement-traceability-matrix)

## 1. Document Overview and HLD Reference

### 1.1 Purpose

This Low-Level Design (LLD) provides implementation-ready engineering specifications for the modules defined in `HLD.md` (`HLD-WASMC-2026-001`). It details concrete data structures, memory layouts, API contracts, SQLite cache schemas, control-flow algorithms, and verification matrices required to construct `wasmc`.

### 1.2 Reference to HLD Modules

Every specification in this document maps directly to the 12 core modules established in Section 3 of `HLD.md`: `MOD-DRV`, `MOD-LEX`, `MOD-PAR`, `MOD-SEM`, `MOD-HIR`, `MOD-MIR`, `MOD-OPT`, `MOD-SCF`, `MOD-STK`, `MOD-MEM`, `MOD-BIN`, and `MOD-BND`.

## 2. Detailed Module and Component Design

### 2.1 Front-End Subsystem (`MOD-LEX`, `MOD-PAR`, `MOD-SEM`)

* **`MOD-LEX` (`Lexer` & `StringInterner`):**

  * Operates over an immutable `&[u8]` buffer using a 256-entry ASCII jump table with UTF-8 multi-byte validation (`FE-LEX-001`).

  * Emits a 12-byte `Token` struct (`kind: TokenKind`, `start: u32`, `len: u32`, `sym: SymbolId`).

  * `StringInterner` uses a bump-allocated byte slab (`ArenaVec<u8>`) and an open-addressed `FxHash` table mapping string slices to sequential `SymbolId(u32)` handles (`FE-LEX-003`).

* **`MOD-PAR` (`Parser` & `AstPool`):**

  * Uses $k=2$ token lookahead (`FE-LEX-004`).

  * Top-level items (`fn`, `struct`, `global`, `import`, `export`) and control statements (`if`, `while`, `for`, `return`, `block`) are parsed via Recursive Descent (`FE-PAR-001`).

  * Expressions use Pratt parsing (`parse_expr_bp(min_bp: u8)`) driven by a static binding-power table (`FE-PAR-002`).

  * Stores nodes in `AstPool` using Struct-of-Arrays (SoA): `nodes: Vec<AstNode>`, `spans: Vec<Span>`, `types: Vec<TypeId>` (`FE-AST-001`, `FE-AST-002`).

* **`MOD-SEM` (`TypeChecker` & `EscapeAnalyzer`):**

  * Scopes are stored in a flat vector `Vec<ScopeFrame>` where each frame holds `parent: Option<ScopeId>` (Cactus Stack, `FE-SEM-001`).

  * Implements `synthesize_expr(node: NodeIndex) -> TypeId` ($\Gamma \vdash e \Rightarrow \tau$) and `check_expr(node: NodeIndex, expected: TypeId) -> TypeId` ($\Gamma \vdash e \Leftarrow \tau$) (`FE-SEM-002`).

  * Unary operator `&` (`AddrOf`) sets `SymbolEntry::flags |= SYM_ESCAPED_TO_MEM` (`FE-SEM-004`, `BR-003`).

### 2.2 Middle-End Subsystem (`MOD-HIR`, `MOD-MIR`, `MOD-OPT`)

* **`MOD-HIR` (`HirLowerer`):**

  * Rewrites `for (init; cond; step) { body }` into canonical `block { init; loop { if !cond break; body; step } }`.

  * Expands generic functions (`fn max<T>(a: T, b: T)`) into concrete monomorphized clones keyed by `(SymbolId, Vec<TypeId>)` (`SRS Sec 4.1`).

* **`MOD-MIR` (`MirBuilder`):**

  * Constructs a CFG of `BasicBlock`s connected by `Terminator` edges (`IR-MIR-003`).

  * Implements **Braun et al.’s on-the-fly SSA construction** (`read_variable` / `write_variable` recursive predecessor lookup), sealing blocks once all predecessors are known and pruning trivial $\phi$-nodes (`IR-MIR-001`).

* **`MOD-OPT` (`MirPassManager`):**

  * Executes deterministic passes in fixed order: `Mem2Reg` $\rightarrow$ `ConstFoldProp` $\rightarrow$ `AlgebraicSimplify` $\rightarrow$ `DominatorVN_CSE` $\rightarrow$ `LeafInliner` ($N \le 25$) $\rightarrow$ `DeadCodeElim` (`IR-OPT-001`..`006`).

### 2.3 Back-End Subsystem (`MOD-SCF`, `MOD-STK`, `MOD-MEM`, `MOD-BIN`, `MOD-BND`)

* **`MOD-SCF` (`Stackifier`):**

  * Computes Cooper-Harvey-Kennedy fast dominance trees and natural loop headers (`CG-SCF-001`).

  * Lowers CFG into nested `WasmBlock` (`Block`, `Loop`, `If`) and computes relative label indices `br $depth` (`CG-SCF-002`).

* **`MOD-STK` (`StackScheduler` & `LocalAllocator`):**

  * Tracks virtual value stack height; elides locals for single-use intra-block SSA values (`CG-STK-001`).

  * Runs a peephole pass fusing `[LocalSet(x), LocalGet(x)]` into `[LocalTee(x)]` (`CG-STK-002`).

  * Performs Linear Scan interval coloring over remaining SSA virtual registers, partitioning locals into `i32`, `i64`, `f32`, and `f64` buckets (`CG-STK-003`).

* **`MOD-MEM` (`LinearMemoryPlanner`):**

  * Reserves `0x00000000`–`0x000003FF` as unmapped guard bytes (`MM-LAY-001`).

  * Packs static strings/constants starting at `0x00000400` up to `__data_end` (`MM-LAY-002`).

  * Sets `__heap_base = align16(__data_end + shadow_stack_size)` (default shadow stack = $1\text{ MiB}$, `MM-LAY-003`..`004`).

  * Injects pre-compiled MIR/LIR for the runtime memory manager (`TLSF` in Manual mode or `MarkSweepGC` in Managed mode, `MM-ALC-001`).

* **`MOD-BIN` (`WasmBinaryWriter`):**

  * Serializes the module preamble and sections in the exact sequence mandated by `CG-BIN-003`: `1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 10, 11, 0`.

## 3. API and CLI Specifications

### 3.1 Command-Line Interface (CLI) Specification

```
USAGE:
    wasmc <SUBCOMMAND> [OPTIONS] <INPUT_FILE>

SUBCOMMANDS:
    build       Compile source file into .wasm (and optional .wat / .js / .d.ts)
    check       Run Front-End (Lexer, Parser, Semantic/Typecheck) without codegen
    daemon      Start the HTTP/2 REST and JSON-RPC incremental build server

OPTIONS:
    -o, --output <PATH>                  Output artifact base path
    --target <TARGET>                    Target ABI: wasm32-unknown-web (default) | wasm32-wasi [SRS 7.1]
    -O, --opt-level <0|1|2|3|s|z>        Optimization level (0=none, 3=full MIR opt) [SRS 8.2]
    --memory-model <manual|gc>           Embedded allocator: manual (TLSF) | gc (Mark-Sweep) [MM-ALC-001]
    --shadow-stack-size <BYTES>          Shadow stack reservation (default: 1048576 / 1 MiB, multiple of 16) [MM-LAY-003]
    --enable-reference-types             Emit native Wasm externref tables for JS objects [RT-JS-003]
    --emit-wat                           Emit disassembled WebAssembly Text (.wat) alongside .wasm [SRS 1.2]
    -g, --debug                          Emit @__wasm_panic hooks, Custom "name" section, and DWARF [RT-TRP-003..004]

```

### 3.2 Compiler Daemon HTTP REST API Specifications (`MOD-DRV`)

#### Endpoint Summary Table

| Endpoint | Method | Auth | Description | Mapped Requirements | 
| ----- | ----- | ----- | ----- | ----- | 
| `/api/v1/compile` | `POST` | Bearer JWT / mTLS | Compiles source payload into `.wasm` + optional `.js`/`.d.ts` bindings. | `FR-001`..`018` | 
| `/api/v1/check` | `POST` | Bearer JWT / mTLS | Performs lexical, syntactic, and semantic validation only; returns diagnostics. | `FR-001`..`006` | 
| `/api/v1/cache/status` | `GET` | Bearer JWT / mTLS | Returns incremental cache hit ratios, artifact counts, and storage metrics. | `FR-018` | 

#### `POST /api/v1/compile` — Request Schema

```
{
  "source_name": "math_engine.src",
  "source_utf8_base64": "Zm4gYWRkKGE6IGkzMiwgYjogbaMyKSA->IGkzMiB7IHJldHVybiBhICsgYjsgfQ==",
  "options": {
    "target": "wasm32-unknown-web",
    "opt_level": "O3",
    "memory_model": "manual",
    "shadow_stack_bytes": 1048576,
    "enable_reference_types": true,
    "debug": false,
    "emit_wat": false
  }
}

```

#### `POST /api/v1/compile` — Response Schema (`200 OK` or `422 Unprocessable Entity`)

```
{
  "status": "SUCCESS",
  "compilation_time_ms": 4.2,
  "sha256_wasm": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "artifacts": {
    "wasm_base64": "AGFzbQEAAAABBgFgAn9/AX8DAgEABwcBA2FkZAAACgkBBwAgACABags=",
    "js_glue_utf8": "export async function instantiate(bytes) { ... }",
    "dts_utf8": "export function add(a: number, b: number): number;"
  },
  "memory_map": {
    "guard_zone_end": 1024,
    "data_end": 1088,
    "heap_base": 1049664,
    "initial_pages_64kib": 17
  },
  "diagnostics": []
}

```

#### HTTP & Application Error Codes

| HTTP Status | Error Code | Trigger Condition | Mapped Requirements | 
| ----- | ----- | ----- | ----- | 
| `400 Bad Request` | `ERR_INVALID_OPTIONS` | Invalid target triple, non-16-byte aligned `--shadow-stack-size`, or malformed JSON. | `MM-ABI-003` | 
| `413 Payload Too Large` | `ERR_SOURCE_EXCEEDS_LIMIT` | Input buffer exceeds configured daemon limit (default 64 MiB). | `FE-LEX-002` | 
| `422 Unprocessable Entity` | `ERR_COMPILATION_FAILED` | Source contains lexical (`FE-LEX-005`), syntactic (`FE-PAR-003`), or semantic (`FE-SEM-002`) errors. Full diagnostic array returned. | `FR-002`, `FR-004`, `FR-005` | 
| `500 Internal Server Error` | `ERR_INTERNAL_COMPILER_BUG` | Unexpected invariant failure (e.g., internal IR verifier mismatch). | `SRS Sec 8.2` | 

## 4. Database Design (Incremental Compilation Cache)

To satisfy sub-100ms compilation turnaround (`SYS-ARCH-001`) and deterministic reproducibility (`NFR-PERF-003`), `wasmc` embeds a local SQLite 3 database (`wasmc_cache.db`) configured with `PRAGMA journal_mode = WAL;` and `PRAGMA synchronous = NORMAL;`.

### 4.1 Entity-Relationship Diagram (ERD)

```
erDiagram
    COMPILATION_UNITS ||--o{ SYMBOL_INDEX : "declares"
    COMPILATION_UNITS ||--o{ CACHED_FUNCTIONS : "contains"
    COMPILATION_UNITS ||--o{ DIAGNOSTIC_LOGS : "emits"

    COMPILATION_UNITS {
        TEXT unit_blake3_hash PK "BLAKE3(source_bytes + compiler_flags)"
        TEXT source_path "Canonical relative file path"
        INTEGER source_byte_len "Size of UTF-8 source buffer"
        TEXT target_triple "wasm32-unknown-web | wasm32-wasi"
        TEXT opt_level "O0 | O1 | O2 | O3 | Os | Oz"
        INTEGER data_end_offset "Computed __data_end address"
        INTEGER heap_base_offset "Computed __heap_base address"
        BLOB wasm_artifact "Full serialized .wasm binary"
        TEXT wasm_sha256 "Deterministic verification digest"
        INTEGER created_at_unix "UTC epoch timestamp"
    }

    CACHED_FUNCTIONS {
        TEXT fn_blake3_hash PK "BLAKE3(HIR_subgraph + opt_level)"
        TEXT unit_blake3_hash FK "Parent compilation unit"
        INTEGER symbol_id "Interned SymbolId"
        TEXT fn_name "Human-readable identifier"
        BLOB mir_ssa_blob "Serialized optimized SSA MIR"
        BLOB wasm_code_payload "Encoded Section 10 function body bytes"
        INTEGER stack_frame_bytes "16-byte aligned shadow stack K"
    }

    SYMBOL_INDEX {
        TEXT unit_blake3_hash FK "Parent compilation unit"
        INTEGER symbol_id "32-bit SymbolId"
        TEXT identifier_str "Deduplicated UTF-8 string"
        INTEGER resolved_type_id "Lowered Wasm TypeId"
        INTEGER storage_class "0=Local, 1=ShadowStack, 2=Global, 3=Func"
        INTEGER is_escaped "Boolean: 1 if &x address taken"
    }

    DIAGNOSTIC_LOGS {
        INTEGER diag_id PK "Auto-increment ID"
        TEXT unit_blake3_hash FK "Parent compilation unit"
        TEXT severity "ERROR | WARNING | INFO"
        TEXT error_code "e.g., E0102_TYPE_MISMATCH"
        INTEGER byte_start "Span start_offset"
        INTEGER byte_len "Span length"
        INTEGER line_num "1-based line coordinate"
        INTEGER col_num "1-based column coordinate"
        TEXT message "Formatted diagnostic message"
    }

```

### 4.2 SQL DDL Schema, Indexes, and Constraints

```
CREATE TABLE IF NOT EXISTS compilation_units (
    unit_blake3_hash TEXT PRIMARY KEY CHECK (length(unit_blake3_hash) = 64),
    source_path      TEXT NOT NULL,
    source_byte_len  INTEGER NOT NULL CHECK (source_byte_len >= 0),
    target_triple    TEXT NOT NULL CHECK (target_triple IN ('wasm32-unknown-web', 'wasm32-wasi')),
    opt_level        TEXT NOT NULL CHECK (opt_level IN ('O0', 'O1', 'O2', 'O3', 'Os', 'Oz')),
    data_end_offset  INTEGER NOT NULL CHECK (data_end_offset >= 1024),
    heap_base_offset INTEGER NOT NULL CHECK (heap_base_offset >= data_end_offset AND heap_base_offset % 16 = 0),
    wasm_artifact    BLOB NOT NULL,
    wasm_sha256      TEXT NOT NULL CHECK (length(wasm_sha256) = 64),
    created_at_unix  INTEGER NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS cached_functions (
    fn_blake3_hash    TEXT PRIMARY KEY CHECK (length(fn_blake3_hash) = 64),
    unit_blake3_hash  TEXT NOT NULL REFERENCES compilation_units(unit_blake3_hash) ON DELETE CASCADE,
    symbol_id         INTEGER NOT NULL,
    fn_name           TEXT NOT NULL,
    mir_ssa_blob      BLOB NOT NULL,
    wasm_code_payload BLOB NOT NULL,
    stack_frame_bytes INTEGER NOT NULL CHECK (stack_frame_bytes >= 0 AND stack_frame_bytes % 16 = 0)
) STRICT;

CREATE INDEX IF NOT EXISTS idx_cached_fn_unit ON cached_functions(unit_blake3_hash, symbol_id);

CREATE TABLE IF NOT EXISTS symbol_index (
    unit_blake3_hash TEXT NOT NULL REFERENCES compilation_units(unit_blake3_hash) ON DELETE CASCADE,
    symbol_id        INTEGER NOT NULL,
    identifier_str   TEXT NOT NULL,
    resolved_type_id INTEGER NOT NULL,
    storage_class    INTEGER NOT NULL CHECK (storage_class BETWEEN 0 AND 3),
    is_escaped       INTEGER NOT NULL CHECK (is_escaped IN (0, 1)),
    PRIMARY KEY (unit_blake3_hash, symbol_id)
) STRICT;

CREATE TABLE IF NOT EXISTS diagnostic_logs (
    diag_id          INTEGER PRIMARY KEY AUTOINCREMENT,
    unit_blake3_hash TEXT NOT NULL REFERENCES compilation_units(unit_blake3_hash) ON DELETE CASCADE,
    severity         TEXT NOT NULL CHECK (severity IN ('ERROR', 'WARNING', 'INFO')),
    error_code       TEXT NOT NULL,
    byte_start       INTEGER NOT NULL,
    byte_len         INTEGER NOT NULL,
    line_num         INTEGER NOT NULL,
    col_num          INTEGER NOT NULL,
    message          TEXT NOT NULL
) STRICT;

CREATE INDEX IF NOT EXISTS idx_diag_unit ON diagnostic_logs(unit_blake3_hash);

```

## 5. Sequence Diagrams for Key Functional Flows

### 5.1 End-to-End Compilation Pipeline (`FR-001` through `FR-013`)

```
sequenceDiagram
    autonumber
    actor Dev as Developer / CI
    participant Drv as MOD-DRV (Driver)
    participant Lex as MOD-LEX (DFA Lexer)
    participant Par as MOD-PAR (RD+Pratt)
    participant Sem as MOD-SEM (Type & Escape)
    participant Mir as MOD-MIR & MOD-OPT
    participant Scf as MOD-SCF & MOD-STK
    participant Bin as MOD-MEM & MOD-BIN

    Dev->>Drv: wasmc build main.src --target wasm32-unknown-web -O3
    Drv->>Lex: tokenize(source_bytes, &mut interner)
    Lex-->>Drv: Vec<Token> + LexErrors (FR-001, FR-002)
    Drv->>Par: parse_module(tokens, &mut ast_pool)
    Note over Par: Panic-mode sync on ';', '}', 'fn'<br/>Injects PoisonExpr on error (FR-004)
    Par-->>Drv: AstPool (Nodes, Spans) + SyntaxErrors (FR-003)
    Drv->>Sem: check_and_analyze(&mut ast_pool, &interner)
    Note over Sem: Bidirectional typing + PoisonType unification<br/>Flags &x as EscapedToLinearMemory (FR-005, FR-006)
    Sem-->>Drv: Typed HIR + EscapeSet + SemErrors
    alt Any Front-End Errors Present
        Drv-->>Dev: Exit 1 with formatted multi-error diagnostics
    else Zero Front-End Errors
        Drv->>Mir: lower_and_optimize(typed_hir, OptLevel::O3)
        Note over Mir: Braun SSA construction -> Mem2Reg -> ConstFold<br/>-> StrengthReduce -> CSE -> Inline -> DCE (FR-007, FR-008)
        Mir-->>Drv: Optimized MirModule
        Drv->>Scf: stackify_and_allocate_locals(mir_module)
        Note over Scf: RPO Dominance Stackifier -> Expression Stackification<br/>-> local.tee fusion -> Linear Scan coloring (FR-009, FR-010)
        Scf-->>Drv: WasmLirModule
        Drv->>Bin: emit_wasm_binary(lir_module, MemoryConfig)
        Note over Bin: Layout 0x0400 data & $__stack_pointer (FR-012, FR-013)<br/>Encode Sections 1..9, 12, 10, 11, 0 via LEB128 (FR-011)
        Bin-->>Drv: Vec<u8> (.wasm) + JS/TS Glue (FR-015)
        Drv-->>Dev: Write artifacts to disk (Exit 0)
    end

```

### 5.2 Host-to-Wasm String & Object Call Flow (`wasm32-unknown-web`, `FR-015`)

```
sequenceDiagram
    autonumber
    participant Host as Browser JS Application
    participant Glue as Generated <module>.js
    participant Mem as Wasm Exported Memory
    participant Wasm as Wasm Module (__alloc / Target Fn)

    Host->>Glue: greetUser("Ada Lovelace", domButtonElement)
    Glue->>Glue: utf8Bytes = TextEncoder.encode("Ada Lovelace") (len=12)
    Glue->>Wasm: ptr = exports.__alloc(12, 1)
    Note over Wasm: TLSF / GC allocates 12 bytes above __heap_base (FR-014)
    Wasm-->>Glue: ptr (i32)
    Glue->>Mem: new Uint8Array(memory.buffer, ptr, 12).set(utf8Bytes)
    Glue->>Glue: refIdx = insertExternRef(domButtonElement)
    Glue->>Wasm: packedRet = exports.greetUser(ptr, 12, refIdx)
    Note over Wasm: Prologue decrements $__stack_pointer by 16B (FR-013)<br/>Processes slice [ptr..ptr+12]<br/>Epilogue restores $__stack_pointer
    Wasm-->>Glue: packedRet (i64: upper 32b = retPtr, lower 32b = retLen)
    Glue->>Mem: outStr = TextDecoder.decode(new Uint8Array(memory.buffer, retPtr, retLen))
    Glue->>Wasm: exports.__dealloc(retPtr, retLen)
    Glue-->>Host: return outStr

```

## 6. Class, Struct, and Interface Definitions

All core data structures obey `SYS-ARCH-002` (`C-002`): zero raw pointers, 32-bit typed indices, and Struct-of-Arrays (SoA) separation.

```
/// Strongly-typed 32-bit arena indices (SYS-ARCH-002)
#[repr(transparent)] pub struct SymbolId(pub u32);
#[repr(transparent)] pub struct NodeIndex(pub u32);
#[repr(transparent)] pub struct TypeId(pub u32);
#[repr(transparent)] pub struct ScopeId(pub u32);
#[repr(transparent)] pub struct BlockId(pub u32);
#[repr(transparent)] pub struct RegId(pub u32);

/// Zero-copy source coordinate span (FE-LEX-002, FE-AST-002)
#[derive(Copy, Clone, Debug)]
pub struct Span {
    pub start_offset: u32,
    pub length: u32,
}

/// Token emitted by the DFA Lexer (FE-LEX-001..005)
#[derive(Copy, Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub symbol: SymbolId, // SymbolId(0) if non-identifier/literal
}

/// Flattened Struct-of-Arrays AST Pool (FE-AST-001, FE-AST-002)
pub struct AstPool {
    pub nodes: Vec<AstNode>,       // Indexed by NodeIndex
    pub spans: Vec<Span>,          // Parallel array indexed by NodeIndex
    pub types: Vec<TypeId>,        // Parallel array populated by MOD-SEM
    pub extra_children: Vec<u32>,  // Contiguous index slices for variadic args/stmts
}

/// 16-byte compact AST Node representation (FE-AST-001)
#[derive(Copy, Clone, Debug)]
pub struct AstNode {
    pub tag: AstTag,               // 1 byte (e.g., BinaryExpr, CallExpr, PoisonExpr, ErrorStmt)
    pub op: u8,                    // 1 byte (Operator or sub-kind)
    pub flags: u16,                // 2 bytes (e.g., ESCAPED_TO_LINEAR_MEM = 1 << 0)
    pub lhs: NodeIndex,            // 4 bytes
    pub rhs: NodeIndex,            // 4 bytes
    pub aux: u32,                  // 4 bytes (SymbolId or slice offset into extra_children)
}

/// Semantic Type Representation with PoisonType support (FE-SEM-002..004)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticType {
    Void,
    I32,
    I64,
    F32,
    F64,
    Pointer { pointee: TypeId, mutable: bool }, // Lowers to i32 in wasm32
    Struct  { name: SymbolId, fields: Vec<(SymbolId, TypeId, u32)>, size: u32, align: u32 },
    Func    { params: Vec<TypeId>, results: Vec<TypeId> },
    Poison, // Unifies silently with any type to suppress cascading errors (FE-SEM-003)
}

/// SSA MIR Instruction & Basic Block (IR-MIR-001..003)
pub struct MirBasicBlock {
    pub id: BlockId,
    pub preds: Vec<BlockId>,
    pub phis: Vec<MirPhi>,
    pub insts: Vec<MirInst>,
    pub terminator: MirTerminator,
}

pub struct MirPhi {
    pub dst: RegId,
    pub ty: WasmValType,
    pub incoming: Vec<(BlockId, RegId)>,
}

pub enum MirInst {
    ConstI32 { dst: RegId, val: i32 },
    ConstI64 { dst: RegId, val: i64 },
    ConstF32 { dst: RegId, bits: u32 }, // Raw IEEE 754 bits (CG-BIN-002)
    ConstF64 { dst: RegId, bits: u64 },
    BinOp    { op: MirBinOp, dst: RegId, lhs: RegId, rhs: RegId },
    Load     { ty: WasmValType, dst: RegId, base: RegId, offset: u32, align: u8 },
    Store    { ty: WasmValType, base: RegId, val: RegId, offset: u32, align: u8 },
    Call     { func_idx: u32, dst: Option<RegId>, args: Vec<RegId> },
    StackAddr { dst: RegId, slot_offset: u32 }, // Address of escaped local on shadow stack
}

pub enum MirTerminator {
    Jump   { target: BlockId },
    Branch { cond: RegId, true_bb: BlockId, false_bb: BlockId },
    Switch { idx: RegId, targets: Vec<BlockId>, default_bb: BlockId },
    Return { val: Option<RegId> },
    Trap   { reason: TrapReason, span: Span },
}

/// Core Pipeline Trait Boundaries
pub trait CompilerPhase<Input, Output> {
    fn execute(&mut self, input: Input, diag: &mut DiagnosticSink) -> Output;
}

```

## 7. Core Algorithms and Business Logic

### 7.1 Pratt Expression Parser with Panic-Mode Recovery (`FE-PAR-001`..`003`, `FR-003`, `FR-004`)

```
ALGORITHM: ParseExpression(lexer, ast_pool, min_bp) -> NodeIndex
1. token := lexer.next()
2. IF token is PrefixOperator(op):
     ((), r_bp) := PrefixBindingPower(op)
     rhs := ParseExpression(lexer, ast_pool, r_bp)
     lhs := ast_pool.alloc_unary(op, rhs, token.span)
   ELSE IF token is PrimaryAtom (Literal, Ident, '('):
     lhs := ParsePrimary(lexer, ast_pool, token)
   ELSE:
     diag.emit_syntax_error(token.span, "Expected expression")
     SynchronizeToTokenSet(lexer, {';', '}', 'fn', 'let', 'return'})
     RETURN ast_pool.alloc_poison_expr(token.span) // FE-PAR-003

3. LOOP:
     op_tok := lexer.peek(0)
     IF op_tok is EOF or Delimiter: BREAK
     IF op_tok is PostfixOperator(op):
       (l_bp, ()) := PostfixBindingPower(op)
       IF l_bp < min_bp: BREAK
       lexer.next()
       lhs := ast_pool.alloc_postfix(op, lhs, op_tok.span)
       CONTINUE
     IF op_tok is InfixOperator(op):
       (l_bp, r_bp) := InfixBindingPower(op)
       IF l_bp < min_bp: BREAK
       lexer.next()
       rhs := ParseExpression(lexer, ast_pool, r_bp)
       lhs := ast_pool.alloc_binary(op, lhs, rhs, Span::merge(lhs, rhs))
       CONTINUE
     BREAK
4. RETURN lhs

```

### 7.2 Dominance-Driven Stackifier Algorithm (`CG-SCF-001`..`002`, `FR-009`)

WebAssembly forbids unstructured jumps (`BR-004`). The Stackifier converts a reducible SSA MIR CFG into structured Wasm `block`, `loop`, and `if` scopes:

```
ALGORITHM: StackifyCFG(mir_fn) -> Vec<WasmLirInstr>
1. VerifyReducibility(mir_fn):
   - Identify back-edges (u -> v) where v does not dominate u.
   - If any loop has multiple entry headers, duplicate body nodes until every loop has a single dominating header (CG-SCF-001).
2. Compute Dominance Tree D and Loop Nesting Forest L of mir_fn.
3. Compute Reverse Post-Order (RPO) sequence B[0..N-1] such that all nodes belonging to a natural loop L_h are contiguous immediately after header h.
4. Initialize ControlStack := [] (entries: { kind: BLOCK|LOOP, target_bb: BlockId }).
5. FOR i := 0 TO N-1:
     bb := B[i]
     // Close any forward blocks whose merge target is the current block `bb`
     WHILE ControlStack.top().kind == BLOCK AND ControlStack.top().target_bb == bb:
       Emit(WasmLir::End)
       ControlStack.pop()
     // If `bb` is a loop header, open a `loop` scope
     IF bb is a LoopHeader in L:
       Emit(WasmLir::Loop(BlockType::Void))
       ControlStack.push({ kind: LOOP, target_bb: bb })
     // Determine forward branch targets from bb or dominated sub-blocks that merge at B[j] (j > i)
     FOR each forward merge target `m` sorted in descending RPO index:
       Emit(WasmLir::Block(BlockType::Void))
       ControlStack.push({ kind: BLOCK, target_bb: m })
     // Emit instructions of `bb` using StackScheduler (eliding single-use locals & fusing local.tee)
     EmitBasicBlockInstructions(bb)
     // Translate terminator branches into relative depth `d`
     FOR each branch target `t` in bb.terminator:
       d := FindRelativeDepth(ControlStack, t) // 0 = innermost enclosing scope
       Emit(br d / br_if d / br_table)
     // If `bb` is the last block in a natural loop `L_h`, close the `loop` scope
     WHILE ControlStack.top().kind == LOOP AND IsLastNodeOfLoop(bb, ControlStack.top().target_bb):
       Emit(WasmLir::End)
       ControlStack.pop()

```

### 7.3 Shadow Stack Prologue, Epilogue & Debug Overflow Check (`MM-ABI-003`, `RT-TRP-003`, `FR-013`, `FR-017`)

For any function whose escaped locals or aggregate structs require $K > 0$ bytes of linear memory stack space:

1. Round $K$ up to a 16-byte multiple: $K_{\text{aligned}} = (K + 15) \land \sim 15$.

2. **Prologue Emission:**

   ```
   global.get $__stack_pointer
   i32.const <K_aligned>
   i32.sub
   local.tee $fp                  ;;$fp holds new frame base (CG-STK-002)
   ;; If --debug is active, emit shadow stack overflow check (RT-TRP-003):
   local.get $fp
   i32.const <__data_end>
   i32.lt_u
   if
     i32.const <msg_ptr>
     i32.const <msg_len>
     i32.const <file_ptr>
     i32.const <line_num>
     call $__wasm_panic           ;; Logs and executes `unreachable` (0x00)
     unreachable
   end
   local.get $fp
   global.set $__stack_pointer
   
   ```

3. **Epilogue Emission (before every `return`):**

   ```
   local.get $fp
   i32.const <K_aligned>
   i32.add
   global.set $__stack_pointer
   
   ```

### 7.4 Bounded LEB128 Serialization (`CG-BIN-001`, `FR-011`)

```
ALGORITHM: EncodeULEB128(value: u64, max_bytes: usize, out: &mut Vec<u8>)
1. count := 0
2. LOOP:
     byte := (value & 0x7F) as u8
     value >>= 7
     IF value != 0:
       byte |= 0x80 // Set MSB continuation bit
     out.push(byte)
     count += 1
     IF value == 0: BREAK
     ASSERT(count < max_bytes) // 5 bytes for u32, 10 bytes for u64

```

## 8. Error Handling, Diagnostics, and Logging Strategy

### 8.1 Multi-Stage Non-Halting Error Recovery

1. **Lexical Recovery (`FE-LEX-005` / `FR-002`):** On invalid UTF-8 bytes or unterminated literals, `MOD-LEX` pushes a `Diagnostic` to `DiagnosticSink`, emits `TokenKind::LexError`, and advances the cursor to the next ASCII whitespace (`0x20`, `0x09`, `0x0A`, `0x0D`).

2. **Syntactic Recovery (`FE-PAR-003` / `FR-004`):** On unexpected tokens, `MOD-PAR` logs the error, skips tokens until encountering `;`, `}`, `fn`, `let`, or `return`, and returns a valid `NodeIndex` pointing to an `AstTag::PoisonExpr` or `AstTag::ErrorStmt` node.

3. **Semantic Suppression (`FE-SEM-003` / `FR-005`):** The unification function `unify(t1, t2)` checks `if t1 == TypeId::POISON || t2 == TypeId::POISON { return Ok(TypeId::POISON); }`. This guarantees a single syntax mistake never triggers a cascade of 50 downstream type-mismatch errors.

4. **Gatekeeper Before Lowering:** If `DiagnosticSink::has_errors()` is true after `MOD-SEM` completes, the driver aborts before `MOD-HIR`/`MOD-MIR` lowering and outputs all collected diagnostics sorted by `(file_id, start_offset)`.

### 8.2 Structured Diagnostic Schema & Logging

* **CLI Output:** Rust-style annotated source snippets with line/column carets (`^^^`) and fix suggestions.

* **Daemon / CI Output:** Structured JSON logs via `tracing` (with trace IDs, compilation phase durations in microseconds, and memory arena high-water marks).

## 9. Security Implementation Details

| Security Domain | Threat / Vulnerability | LLD Implementation Control | Mapped Requirements | 
| ----- | ----- | ----- | ----- | 
| **Parser & Fuzz Resilience** | Maliciously nested expressions (`((((...))))`) causing host stack overflow in Recursive Descent/Pratt parser. | Enforce a hard recursion depth counter (`MAX_PARSE_DEPTH = 512`) in `MOD-PAR`; exceeding the depth emits a structured diagnostic and triggers panic-mode synchronization (`SRS Sec 8.2`). | `FE-PAR-001`..`003`, `SRS Sec 8.2` | 
| **Linear Memory Null Dereference** | Dereferencing an uninitialized or null pointer (`0x0`) corrupting static globals. | Leave bytes `0x00000000`–`0x000003FF` ($1024\text{ B}$) unmapped (`MM-LAY-001`); in `--debug` mode, inject `if (ptr < 1024) call $__wasm_panic` before dynamic loads/stores. | `MM-LAY-001`, `RT-TRP-003`, `BR-006` | 
| **Shadow Stack / Static Data Collision** | Deep recursion overflowing `$__stack_pointer` downward into `.ro`/`.bss` static data below `__data_end`. | Place the shadow stack above `__data_end` growing downward (`MM-LAY-003`) and emit explicit `$fp < __data_end` trap guards in `--debug` builds (`RT-TRP-003`). | `MM-LAY-003`, `RT-TRP-003`, `FR-017` | 
| **Daemon API Security** | Unauthorized remote code compilation or DoS via zip-bombs/huge payloads on `wasmc-daemon`. | Enforce mTLS / Bearer JWT validation on `/api/v1/*` endpoints, cap request payload size at 64 MiB, and execute compilation jobs inside time-bounded worker threads (`timeout = 5000ms`). | `HLD Sec 6 & 7` | 

## 10. Testing Considerations and Scenario Mapping

In accordance with `SRS Section 8.2` (Verification & Validation Matrix), the QA suite is organized into four automated tiers:

| Test Tier | Tooling / Harness | Target Modules | Verification Scenarios & Pass Criteria | Mapped Requirements | 
| ----- | ----- | ----- | ----- | ----- | 
| **1. Unit & Property Tests** | Rust `cargo test` + `proptest` | `MOD-LEX`, `MOD-PAR`, `MOD-SEM`, `MOD-BIN` | Verify zero heap allocations in `Lexer::next_token()` (`FR-001`); verify `PoisonType` suppresses secondary errors (`FR-005`); verify round-trip `uLEB128`/`sLEB128` boundary values (`0`, `i32::MIN`, `i64::MAX`, `FR-011`). | `FR-001`..`006`, `FR-011` | 
| **2. Spec Bytecode Validation** | WABT `wasm-validate` & Bytecode Alliance `wasm-tools validate` | `MOD-SCF`, `MOD-STK`, `MOD-BIN` | **100% of emitted `.wasm` binaries** across all test programs must pass strict W3C section ordering, LEB128 framing, and operand stack-type balance validation (`SRS Sec 8.2.1`). | `FR-009`..`013`, `BR-004`, `BR-005` | 
| **3. Differential Execution Suite** | **Wasmtime** (AOT/JIT) vs. **Node.js / V8** | `MOD-OPT`, `MOD-MEM`, `MOD-BND` | Compile arithmetic, shadow-stack struct passing, TLSF/GC allocation, and control-flow benchmarks at `-O0` and `-O3`; assert identical stdout and exit codes across Wasmtime and V8 (`SRS Sec 8.2.2`). | `FR-008`, `FR-012`..`017` | 
| **4. Coverage-Guided Fuzzing & CI Benchmarks** | `cargo-fuzz` (`libFuzzer`) + Cross-OS CI Matrix (Linux, macOS, Windows) | `MOD-LEX`, `MOD-PAR`, `MOD-SCF`, `MOD-DRV` | Fuzz Lexer, Parser, and Stackifier with random byte streams to prove zero compiler panics or infinite loops (`SRS Sec 8.2.3`); verify SHA-256 bit-for-bit equality across OS hosts, $\ge 50\text{k LOC/s}$ speed, and $< 512\text{ B}$ minimal web binary (`FR-018`). | `FR-002`, `FR-004`, `FR-009`, `FR-018` | 

## 11. Open Questions

1. **OQ-LLD-001 (Concrete Grammar Syntax - `OI-001`):** Section 6 defines AST tags assuming a Rust/C-like syntax (`fn`, `let`, `struct`, `&x`). Stakeholders must confirm if any additional domain-specific operators or custom precedence levels are required in `InfixBindingPower`.

2. **OQ-LLD-002 (GC Root Map Format in Mode B - `MM-ALC-001`):** For **Mode B** (Mark-and-Sweep GC), our LLD spills live pointer locals directly onto the shadow stack frame prior to function calls so the GC can scan `[$__stack_pointer .. __heap_base]` conservatively/precisely. Confirm if a separate static stack-map table in `.ro` data is preferred over shadow-stack spilling.

3. **OQ-LLD-003 (WASI Preview 2 Component Model Wrapping):** `SRS Title` mentions WASI Preview 1/2 while `SRS Section 7.3` specifies `wasi_snapshot_preview1` (`_start`, `fd_write`). Confirm whether WASI Preview 2 requires native Component Model binary wrapping or standard Preview 1 core module imports with an adapter.

## 12. LLD Requirement Traceability Matrix

| SRS Requirement ID | FRD Requirement ID | LLD Section(s) | Concrete LLD Artifact / Struct / Algorithm | 
| ----- | ----- | ----- | ----- | 
| `SYS-ARCH-001` | `BR-001`, `FR-018` | Sec 2.3, 3.1 | `WasmBinaryWriter` direct emission (zero LLVM/Binaryen) | 
| `SYS-ARCH-002` | `C-002`, `FR-018` | Sec 2.1, 6 | `SymbolId(u32)`, `NodeIndex(u32)`, `AstPool` SoA vectors | 
| `FE-LEX-001`..`004` | `FR-001` | Sec 2.1, 6 | `Token`, `Span`, `StringInterner` (Arena + `FxHash`) | 
| `FE-LEX-005` | `FR-002` | Sec 2.1, 8.1 | `TokenKind::LexError` + whitespace delimiter recovery | 
| `FE-PAR-001`..`002` | `FR-003` | Sec 2.1, 7.1 | `ParseExpression` Pratt algorithm + Recursive Descent | 
| `FE-PAR-003` | `FR-004` | Sec 2.1, 7.1, 8.1 | `SynchronizeToTokenSet` + `AstTag::PoisonExpr` | 
| `FE-AST-001`..`002` | `FR-003` | Sec 2.1, 6 | `AstPool` (`nodes`, `spans`, `types` parallel arrays) | 
| `FE-SEM-001`..`003` | `FR-005` | Sec 2.1, 6, 8.1 | `ScopeId` cactus stack, `SemanticType::Poison` unification | 
| `FE-SEM-004` | `FR-006`, `BR-002`, `BR-003` | Sec 2.1, 6 | `ESCAPED_TO_LINEAR_MEM` flag on `&x` (`AddrOf`) | 
| `IR-MIR-001`..`003` | `FR-007` | Sec 2.2, 6 | `MirBasicBlock`, `MirPhi`, `MirInst`, `MirTerminator` | 
| `IR-OPT-001`..`006` | `FR-008` | Sec 2.2, 5.1 | `MirPassManager` (`Mem2Reg`, `ConstFoldProp`, `CSE`, `DCE`, `LeafInliner`) | 
| `CG-SCF-001`..`002` | `FR-009`, `BR-004` | Sec 2.3, 7.2 | `StackifyCFG` algorithm (RPO + Dominance + `ControlStack`) | 
| `CG-STK-001`..`003` | `FR-010` | Sec 2.3, 5.1 | `StackScheduler`, `LocalTee` peephole fusion, Linear Scan coloring | 
| `CG-BIN-001`..`003` | `FR-011`, `BR-005` | Sec 2.3, 7.4 | `EncodeULEB128`, IEEE 754 bit preservation, Section `1..9, 12, 10, 11, 0` serializer | 
| `MM-LAY-001`..`004` | `FR-012`, `BR-006` | Sec 2.3, 3.2, 9 | `LinearMemoryPlanner` (`0x0400` data, `$__stack_pointer`, `__heap_base`) | 
| `MM-ABI-001`..`003` | `FR-013`, `BR-007` | Sec 4.2, 7.3 | 16-byte aligned `$fp` Prologue/Epilogue emission | 
| `MM-ALC-001` | `FR-014` | Sec 2.3, 3.1 | \`--memory-model <manual | 
| `RT-JS-001`..`003` | `FR-015` | Sec 3.2, 5.2 | ES6 `.js`/`.d.ts` generator, `__alloc`/`__dealloc`, `TextEncoder`/`externref` flow | 
| `RT-WASI-001`..`002` | `FR-016` | Sec 2.3, 3.1 | `_start` synthesis and `wasi_snapshot_preview1` (`fd_write` `ciovec`) lowering | 
| `RT-TRP-001`..`004` | `FR-017`, `BR-008` | Sec 2.3, 7.3, 9 | `0xFC` saturating casts, `@__wasm_panic` intrinsic, Custom `"name"` section | 
| `NFR-PERF-001`..`003` | `FR-018` | Sec 4, 10 | SQLite WAL incremental cache (`compilation_units`), SHA-256 verification, fuzz suite | 
