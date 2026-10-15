# Implementation Gap Analysis: Direct-to-WebAssembly Compiler (`wasmc`)

## Document Metadata
- **Project:** Direct-to-WebAssembly Systems Compiler (`wasmc`)
- **Date:** October 15, 2026
- **Status:** Approved Baseline
- **Governing Precedence:** SDD > SRS > FRD > HLD > LLD

---

## 1. Executive Summary

This gap analysis reviews the five authoritative project specifications (`direct_to_wasm_compiler_srs.md`, `direct_to_wasm_compiler_sdd.md`, `direct_to_wasm_compiler_lld.md`, `direct_to_wasm_compiler_hld.md`, and `direct_to_wasm_compiler_frd.md`). It identifies textual contradictions, incomplete specifications, scope boundary variances, and technology requirements, establishing concrete engineering resolutions before workspace implementation begins.

---

## 2. Reconciled Specification Issues & Resolutions

### Gap 1: Formal Source Language Grammar & Type System Scope
- **Source Documents:** FRD Section 1 (OI-001), HLD Section 10 (OQ-001), LLD Section 11 (OQ-001), SDD Section 4.1 & Section 9 (ISS-001).
- **Issue:** The SRS and FRD called for composite types, pointers, arrays, and generics, but left the concrete source language syntax underspecified and raised open questions regarding whether v1.0 must support C++-style multiple inheritance and virtual function tables.
- **Impact:** Ambiguity in the AST, parser, and semantic typing passes.
- **Resolution:** Adopt the authoritative formal EBNF grammar defined in **SDD Section 4.1**:
  - Top-level items: `import`, `export`, `struct`, `global`, `fn`.
  - Type system: `i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`, `char`, `void`, pointer types `*T`, fixed arrays `[T; N]`, function pointers `fn(...) -> T`, and nominal `struct` types.
  - Dispatch: Indirect calls via Wasm `call_indirect` (Section 4 Table & Section 9 Element). C++-style multiple-inheritance vtables are deferred to v2.0 per SDD ADR-005.
- **Verification Plan:** Unit tests for all grammar productions (`tests/parser/`), semantic tests for type lowering, and indirect call execution tests in Node.js/V8.

---

### Gap 2: WebAssembly Section Ordering & Section 12 (DataCount)
- **Source Documents:** SRS Section 5.3 (`CG-BIN-003`), SDD Section 3.2 (`COMP-SER`) & Section 4.3 (`SectionId`), LLD Section 2.3 (`MOD-BIN`).
- **Issue:** Standard WebAssembly specifications strictly require numeric ascending order for sections 1 to 11, with the exception of the DataCount section (ID 12) introduced in bulk memory operations, which MUST precede the Code section (ID 10) if emitted, and Custom sections (ID 0) which may appear at the beginning or end.
- **Impact:** If sections are emitted out of order, any standard WebAssembly validator (`WebAssembly.validate`, `wasm-validate`) will reject the module with a malformed section error.
- **Resolution:**
  - Standard section sequence:
    1. Type (1)
    2. Import (2)
    3. Function (3)
    4. Table (4)
    5. Memory (5)
    6. Global (6)
    7. Export (7)
    8. Start (8)
    9. Element (9)
    10. DataCount (12) — emitted only when data segments are referenced or required
    11. Code (10)
    12. Data (11)
    13. Custom (0) — names, DWARF, and metadata appended after standard sections.
  - Omit empty sections entirely per `BR-007`.
- **Verification Plan:** Verify byte-level section headers and enforce validation with `WebAssembly.validate` in Node.js and internal binary verifier unit tests.

---

### Gap 3: Exception Handling and Tail Call Proposal Encodings
- **Source Documents:** SDD Section 3.7 (ADR-004), Section 4.2.6 (`WasmInst`), SRS Section 7.4.
- **Issue:** Earlier drafts referred to legacy JS-emulated exception unwinding and loop rewriting. The consolidated SDD standardizes on native WebAssembly Core 2.0 proposals: `try_table` (opcode `0x1F`) with catch clauses, `throw` (`0x08`), `return_call` (`0x12`), and `return_call_indirect` (`0x13`).
- **Impact:** Emitting proposal opcodes requires modern engine support (e.g. Node.js v22+, V8 v123+, Wasmtime v20+).
- **Resolution:**
  - Implement native `try_table`, `throw`, `return_call`, and `return_call_indirect`.
  - Provide configurable flags `--enable-exceptions` (default: true) and `--enable-tail-calls` (default: true) per SDD CLI specification.
  - Node.js v24.19.0 installed in the environment supports these proposals natively.
- **Verification Plan:** Execute recursive functions with `return tail` exceeding $10^5$ call depth to prove $O(1)$ stack behavior, and run `try/catch` execution tests.

---

### Gap 4: Control-Flow Structuring: Stackifier vs Relooper
- **Source Documents:** SRS Section 5.1 (`CG-SCF-002`) vs FRD Section 4.3 (`FR-008`, `BR-003`) and SDD Section 3.7 (ADR-001).
- **Issue:** SRS text mentioned "Relooper or Stackifier", while the FRD and SDD explicitly ban Relooper's synthetic state variables (`BR-003`).
- **Impact:** Synthetic dispatch variables degrade performance and inflate bytecode size.
- **Resolution:** Use exclusively the **Stackifier** algorithm with Tarjan SCC loop normalization and dominance-tree analysis. Irreducible loops are converted to reducible natural loops via node cloning. No state-machine dispatch loops or synthetic `label` variables will be emitted.
- **Verification Plan:** Unit tests with nested `while`, `break`, `continue`, and early return control graphs; verify zero synthetic local allocations in the emitted Wasm bytecode.

---

### Gap 5: Linear Memory Layout, Shadow Stack, and Heap Allocator
- **Source Documents:** SRS Section 6.1–6.3, SDD Section 4.4, SDD Section 9 (ISS-002, ISS-003).
- **Issue:** The relationship between static data, shadow stack, and heap, as well as whether an allocator is bundled or external, needed harmonization.
- **Resolution:**
  - `0x00000000` – `0x000003FF`: 1024-byte unmapped null-pointer guard zone.
  - `0x00000400` – `__data_end`: Static data (strings, constants, globals), naturally aligned.
  - `__data_end` – `__heap_base`: Downward-growing shadow stack (default 1 MiB = 1,048,576 bytes).
  - Global `#0`: `__stack_pointer` (mutable `i32`) initialized to `__heap_base`.
  - Functions allocating stack space emit frame prologue: `global.get $__stack_pointer`, `i32.sub $K`, `local.set $fp`, `global.set $__stack_pointer`.
  - Epilogue: `global.set $__stack_pointer (local.get $fp + $K)`.
  - Stack overflow check: `if ($fp < DATA_END) { unreachable }` emitted by default across all optimization levels (opt-out via `--no-stack-check`).
  - Dynamic heap: starts at `__heap_base`, expanding upward with `memory.grow`.
  - Memory manager: Bundled segregated free-list allocator (`__wasmc_alloc`, `__wasmc_free`) above `__heap_base` enabled by default (opt-out via `--no-default-allocator`).
- **Verification Plan:** Memory layout boundary assertions, struct address-taken pointer tests, and dynamic memory allocation integration tests.

---

### Gap 6: CLI and Daemon REST / JSON-RPC Interfaces
- **Source Documents:** SRS Section 7.1, SDD Section 4.6, LLD Section 3.
- **Issue:** Minor parameter naming differences between CLI specifications (e.g. `--target wasm32-unknown-web` vs `browser-ffi`, `-O0..-Oz`, `--output`).
- **Resolution:**
  - CLI: Accept both standard target names (`wasm32-unknown-web` and `browser-ffi` as aliases; `wasm32-wasi` and `wasi-preview1` as aliases).
  - Subcommands: `wasmc build <INPUT>`, `wasmc check <INPUT>`, `wasmc daemon`.
  - Options: `-o, --output`, `-t, --target`, `-O, --opt-level`, `-g, --debug`, `--emit-wat`, `--shadow-stack-size`, `--no-default-allocator`, `--no-stack-check`.
  - Daemon endpoints: `POST /api/v1/compile`, `POST /api/v1/check` (alias `/api/v1/validate`), `GET /api/v1/cache/status`, `GET /api/v1/health`.
- **Verification Plan:** CLI argument parsing integration tests and Axum HTTP endpoint automated tests.

---

### Gap 7: Incremental Compilation Cache
- **Source Documents:** SDD Section 4.5, LLD Section 4, SRS Section 8.1.
- **Issue:** Deriving robust cache keys that prevent stale artifact reuse while allowing sub-100ms warm rebuilds.
- **Resolution:** Derive cache keys from:
  `BLAKE3(source_content_utf8 || compiler_version || target_triple || opt_level || shadow_stack_size || debug_flag || feature_flags)`.
  Use embedded SQLite with WAL mode (`rusqlite` bundled) and transaction-safe schema migrations.
- **Verification Plan:** Cache hit/miss tests, invalidation upon source modification, invalidation upon flag changes, and byte-for-byte reproducibility tests.
