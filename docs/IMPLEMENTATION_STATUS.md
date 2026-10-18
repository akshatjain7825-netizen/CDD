# Implementation Status & Traceability Matrix — `wasmc`

> **Living Docs Role: Live Status**
> Governed under ECC `living-docs-governance`. This document reflects the authoritative implementation status, verified crate requirements, diagnostic error inventories, and active test coverage across the 16 workspace crates.

## 1. Compliance Classification

In accordance with Section 20 of the engineering specification, all requirements have been audited, classified, and verified against the Rust 2024 implementation:

- **Overall Workspace Test Suite:** **30 / 30 PASS (100%)**
- **Automated Dry Run & V8 Execution:** **39 / 39 PASS (100%)**
- **Compiler Warnings:** **0 Warnings across all 16 crates**

---

## 2. Requirement Traceability Matrix

| Requirement ID | Specification Document & Section | Description | Implementation Crate | Verification Status | Verification Evidence |
|---|---|---|---|---|---|
| **REQ-FE-001** | SRS §4.1 / SDD §3.1 | Deterministic UTF-8 zero-copy Lexer & SymbolInterner | `wasmc-lexer` | **Implemented and verified** | `test_lexer_tokenization_and_recovery` in `crates/wasmc-driver/tests/integration_tests.rs` |
| **REQ-FE-002** | SRS §4.2 / SDD §3.2 | Panic-mode lexical error recovery (`E1001`–`E1003`) | `wasmc-lexer` | **Implemented and verified** | Passes invalid character test `@` without aborting pipeline |
| **REQ-FE-003** | SRS §4.3 / SDD §3.3 | Pratt operator-precedence expression parsing & RD declarations (`E2001`) | `wasmc-parser` | **Implemented and verified** | `test_parser_operator_precedence` |
| **REQ-FE-004** | SRS §4.4 / SDD §3.4 | Flat arena-allocated AST Pool (`NodeIndex(u32)`) | `wasmc-ast` | **Implemented and verified** | AstPool allocations across all parses |
| **REQ-SE-001** | SRS §5.1 / SDD §4.1 | Lexical scoping, symbol tables, duplicate detection (`E3001`) | `wasmc-semantics` | **Implemented and verified** | `test_semantics_duplicate_symbol_error` |
| **REQ-SE-002** | SRS §5.2 / SDD §4.2 | Bidirectional typechecking & error suppression (`E3002`–`E3013`) | `wasmc-semantics` | **Implemented and verified** | `test_semantics_type_mismatch_error`, `test_semantics_unresolved_symbol_error` |
| **REQ-SE-003** | SRS §5.3 / SDD §4.3 | Address-taken escape analysis (`StorageClass::VirtualLocal` vs `ShadowStack`) | `wasmc-semantics` | **Implemented and verified** | Compiled and tested `examples/memory.src` (`stack_math` == 72) |
| **REQ-IR-001** | SRS §6.1 / SDD §5.1 | High-level IR canonicalization and loop desugaring | `wasmc-hir` | **Implemented and verified** | `for`/`while` canonicalization into normalized conditional blocks |
| **REQ-IR-002** | SRS §6.2 / SDD §5.2 | Braun et al. on-the-fly SSA construction & phi placement | `wasmc-mir` | **Implemented and verified** | SSA generation across all example programs |
| **REQ-OPT-001**| SRS §7.1 / SDD §6.1 | Constant folding, propagation & branch simplification | `wasmc-optimizer` | **Implemented and verified** | Verified in optimization pass manager (-O0..-O3) |
| **REQ-OPT-002**| SRS §7.2 / SDD §6.2 | Algebraic simplification & strength reduction (`x * 2 -> x + x`) | `wasmc-optimizer` | **Implemented and verified** | Verified in `pass_algebraic_simplification` |
| **REQ-OPT-003**| SRS §7.3 / SDD §6.3 | Common Subexpression Elimination (CSE) | `wasmc-optimizer` | **Implemented and verified** | Verified in `pass_cse` (including `StackAddr` deduplication) |
| **REQ-OPT-004**| SRS §7.4 / SDD §6.4 | Dead code & unreachable block elimination | `wasmc-optimizer` | **Implemented and verified** | Verified in `pass_dce` |
| **REQ-OPT-005**| SDD §6.5 | Copy propagation & transitive VReg alias elimination | `wasmc-optimizer` | **Implemented and verified** | `benchmark-optimization-loop` verified -34 B in `sorting.src` |
| **REQ-STK-001**| SRS §8.1 / SDD §7.1 | Tarjan SCC natural loop detection & RPO traversal | `wasmc-stackifier` | **Implemented and verified** | `test_end_to_end_control_flow_and_loops` |
| **REQ-STK-002**| SRS §8.2 / SDD §7.2 | Stackifier structured block synthesis (`block`, `loop`, `if`, relative `br`/`br_if`) | `wasmc-stackifier` | **Implemented and verified** | Node.js V8 execution of `sum_up_to` (55) & `count_even` (5) |
| **REQ-MEM-001**| SRS §9.1 / SDD §8.1 | Linear memory planner (1024-byte null guard, static data, downward shadow stack, heap base) | `wasmc-memory` | **Implemented and verified** | Verified memory map layout & `__stack_pointer` initialization |
| **REQ-WASM-001**| SRS §10.1 / SDD §9.1 | Bounded canonical LEB128 codec (u32, i32) | `wasmc-wasm` | **Implemented and verified** | `test_leb128_codec_boundaries` |
| **REQ-WASM-002**| SRS §10.2 / SDD §9.2 | Direct binary serialization of sections 1..11 + custom 0 | `wasmc-wasm` | **Implemented and verified** | Byte-level header validation (`\0asm\1\0\0\0`) & section framing |
| **REQ-WASM-003**| SRS §10.3 / SDD §9.3 | Diagnostic WAT text serialization (`--emit-wat`) | `wasmc-wasm` | **Implemented and verified** | Generated WAT files for all 6 examples |
| **REQ-WASM-004**| SRS §10.4 / SDD §9.4 | Iterative Peephole optimization (`local.set` + `local.get` -> `local.tee`, no-op folds) | `wasmc-wasm` | **Implemented and verified** | Verified in WAT dumps of `arithmetic.wat` and `control_flow.wat` |
| **REQ-BIND-001**| SRS §11.1 / SDD §10.1| ES6 JavaScript glue & TypeScript `.d.ts` generator | `wasmc-bindings` | **Implemented and verified** | Bindings generated in compilation artifacts |
| **REQ-BIND-002**| SRS §11.2 / SDD §10.2| WASI Preview 1 runner shim generator | `wasmc-bindings` | **Implemented and verified** | Generated WASI runtime runner shim |
| **REQ-CACHE-001**| SRS §12.1 / SDD §11.1| SQLite 3 WAL content-addressable compilation cache with `busy_timeout` | `wasmc-cache` | **Implemented and verified** | `test_incremental_cache_sqlite`, `test_sqlite_cache_lifecycle_and_wal_timeout` |
| **REQ-CACHE-002**| SRS §12.2 / SDD §11.2| BLAKE3 version, target, opt-level, and feature-flag extended hashing | `wasmc-cache` | **Implemented and verified** | `test_flags_hash_sensitivity` |
| **REQ-CLI-001** | SRS §13.1 / SDD §12.1| Native CLI executable (`build`, `check`, `daemon`) | `wasmc-cli` | **Implemented and verified** | Executed CLI commands on Windows x86_64 |
| **REQ-SRV-001** | SRS §14.1 / SDD §13.1| Axum HTTP/2 REST daemon with `spawn_blocking`, 1 MB limit & 10s timeout | `wasmc-daemon` | **Implemented and verified** | `test_daemon_health`, `test_daemon_compile_payload_too_large`, etc. |
| **REQ-DET-001** | SRS §15.1 / SDD §14.1| Byte-for-byte deterministic builds | `wasmc-driver` | **Implemented and verified** | `test_deterministic_byte_reproducibility` |

---

## 3. Authoritative Compiler Invariants

### 3.1 Error Code Registry
- **Daemon / Network**: `E0001` (Payload > 1 MB), `E0002` (Worker thread join error), `E0003` (Compilation timeout > 10s).
- **Lexer**: `E1001` (Unrecognized character token), `E1002` (Unterminated string literal), `E1003` (Invalid numeric escape).
- **Parser**: `E2001` (Syntax parse error, unexpected token, unclosed delimiter).
- **Semantics**: `E3001` (Duplicate symbol), `E3002` (Type mismatch), `E3003` (Unresolved symbol), `E3004` (Immutability violation), `E3005` (Argument count mismatch), `E3006` (Invalid return type), `E3007` (Break/continue outside loop), `E3008` (Non-pointer dereference), `E3009` (Non-struct field access), `E3010` (Unknown struct field), `E3011` (Non-indexable array access), `E3012` (Non-integer array index), `E3013` (Invalid type cast).
- **Codegen**: `E5001` (Invalid function index), `E5002` (Missing function index in indirect dispatch).

### 3.2 Storage Classification
- `StorageClass::VirtualLocal { local_idx }`: Unescaped local variables mapped directly to WebAssembly function locals or SSA virtual registers.
- `StorageClass::ShadowStack { frame_offset, size_bytes, align_bytes }`: Address-taken variables (escaped via `&`) allocated in downward-growing linear memory.
- `StorageClass::GlobalSlot { global_idx, is_mut }`: Package-level globals stored in WebAssembly globals or linear memory data segments.
- `StorageClass::FunctionRef { func_idx }`: Function references for direct calls and table element indices.

### 3.3 Daemon REST API Surface
- `GET /api/v1/health`: Liveness probe returning `{"status": "UP"}`.
- `POST /api/v1/compile`: Dispatches compilation into `spawn_blocking`, enforces 1 MB body limit and 10s timeout, returns base64 `.wasm`, BLAKE3 hash, duration, and memory map layout.
- `POST /api/v1/check` (alias `/api/v1/validate`): Typechecks source payload, returns diagnostic sink messages without bytecode emission.
- `GET /api/v1/cache/status`: Returns SQLite 3 cache metrics (`total_units`, `total_functions`, `total_sessions`, `db_size_bytes`).

