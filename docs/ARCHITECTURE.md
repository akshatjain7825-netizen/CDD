# Architecture Specification — Direct-to-WebAssembly Compiler (`wasmc`)

## 1. System Overview

`wasmc` is an optimizing, direct-to-binary WebAssembly compiler toolchain written in pure Rust (2024 edition). It compiles a statically typed imperative systems language directly into valid WebAssembly (`.wasm`) binary modules without relying on LLVM, Binaryen, C/C++, Emscripten, or external assemblers.

```
Source Code (.src)
       │
       ▼
 ┌────────────┐
 │   Lexer    │ ───► Zero-copy Token stream + SymbolInterner (u32 IDs)
 └────────────┘
       │
       ▼
 ┌────────────┐
 │   Parser   │ ───► Pratt expression parsing + RD declarations ──► Flattened Arena AST
 └────────────┘
       │
       ▼
 ┌────────────┐
 │  Semantic  │ ───► Scoping, Bidirectional Typing, Escape Analysis
 │  Analyzer  │      StorageClass: VirtualLocal vs ShadowStack
 └────────────┘
       │
       ▼
 ┌────────────┐
 │ HIR Lower  │ ───► Canonicalization, while/for desugaring, compound op lowering
 └────────────┘
       │
       ▼
 ┌────────────┐
 │ MIR Lower  │ ───► Braun et al. on-the-fly SSA construction, BasicBlocks, VRegs, Phis
 └────────────┘
       │
       ▼
 ┌────────────┐
 │ Optimizer  │ ───► Constant Folding/Propagation, Algebraic Simplification, CSE, DCE
 └────────────┘
       │
       ▼
 ┌────────────┐
 │ Stackifier │ ───► Tarjan SCC Loop analysis, RPO traversal, block/loop/br/br_if synthesis
 └────────────┘
       │
       ▼
 ┌────────────┐
 │  Codegen   │ ───► Wasm opcodes, Peephole local.tee, Shadow Stack frame setup
 └────────────┘
       │
       ▼
 ┌────────────┐
 │ Binary     │ ───► Direct Section Serialization (1..11 + 0), Bounded LEB128 codec
 │  Emitter   │
 └────────────┘
       │
       ▼
 WebAssembly Module (.wasm) + Host Bindings (.js / .d.ts)
```

---

## 2. Workspace Crate Architecture

The compiler is organized as a modular Rust Cargo workspace:

| Crate | Responsibility |
|---|---|
| `wasmc-diagnostics` | Source location spans (`SourceSpan`), rich error codes (`E1001`..`E5002`), colored terminal and JSON diagnostics sink. |
| `wasmc-lexer` | Handwritten deterministic UTF-8 lexer, zero-copy token spans, 32-bit string symbol interner (`SymbolInterner`). |
| `wasmc-ast` | Flat arena-allocated AST (`AstPool`, `NodeIndex(u32)`), parallel source span mapping, zero recursive heap allocations. |
| `wasmc-parser` | Recursive descent declaration/statement parser, Pratt operator-precedence expression parser, panic-mode phrase synchronization on `;` and `}`. |
| `wasmc-semantics` | Lexical scoping, bidirectional typechecker, `Poison` error-suppression, address-taken escape analysis assigning `StorageClass::VirtualLocal` vs `StorageClass::ShadowStack`. |
| `wasmc-hir` | High-level Intermediate Representation desugaring loops, compound assignments, and logical operators. |
| `wasmc-mir` | Mid-level Control-Flow Graph (`BasicBlock`), typed virtual registers (`VReg`), explicit terminators, Braun et al. SSA construction, phi placement. |
| `wasmc-optimizer` | Configurable multi-pass optimization pipeline: constant folding, algebraic simplification, strength reduction, common subexpression elimination (CSE), dead-code elimination (DCE). |
| `wasmc-stackifier` | Tarjan strongly-connected-components (SCC) loop detection, Reverse Post-Order (RPO) CFG traversal, WebAssembly structured control flow generator (`block`, `loop`, `if`, relative `br`/`br_if` depths). |
| `wasmc-memory` | Deterministic linear memory planner, 1024-byte null-pointer trap region, static data, downward-growing shadow stack, heap base calculation. |
| `wasmc-wasm` | Bounded canonical LEB128 codec, Wasm Core 2.0 binary section framing (Sections 1..11, custom 0), direct binary emitter, and diagnostic WAT text serializer. |
| `wasmc-bindings` | ES6 JavaScript module and TypeScript `.d.ts` declaration generator for browser target; Node.js WASI preview 1 shim generator. |
| `wasmc-cache` | Content-addressable SQLite 3 build cache using WAL mode, BLAKE3 dependency hashing, session tracking, and cache invalidation. |
| `wasmc-driver` | End-to-end compilation orchestrator coordinating passes, caching, file I/O, diagnostics reporting, and artifact packaging. |
| `wasmc-daemon` | High-performance Axum HTTP/2 daemon exposing REST endpoints `/api/v1/compile`, `/api/v1/check`, `/api/v1/cache/status`, `/api/v1/health`. |
| `wasmc-cli` | Native CLI binary `wasmc` providing `build`, `check`, and `daemon` subcommands. |

---

## 3. Intermediate Representation Pipeline

### 3.1 AST Pool Representation
The AST uses an indexed flat array (`Vec<AstNode>`) indexed by `NodeIndex(u32)`. Spans are stored in a contiguous parallel array `Vec<SourceSpan>`, ensuring zero cache-miss pointer chasing during syntactic and semantic passes.

### 3.2 High-Level IR (HIR)
Desugars high-level control structures:
- `for (init; cond; post) body` desugars into `init; while (cond) { body; post; }`
- Compound assignments (`x += y`) expand into explicit binary evaluations and stores.
- Short-circuit boolean expressions (`a && b`, `a || b`) lower into explicit conditional branching.

### 3.3 Mid-Level IR (MIR) & SSA Construction
Functions are represented as explicit Control Flow Graphs (`BasicBlock`) terminated strictly by `Terminator::Jump`, `Terminator::Branch`, `Terminator::Return`, or `Terminator::Unreachable`. SSA form is constructed on-the-fly using the Braun et al. algorithm:
- Trivial phis are removed dynamically.
- Incomplete phis for backward loop edges are tracked and sealed upon completing loop bodies.
- Virtual registers (`VReg`) are assigned explicit types (`MirType`).

### 3.4 Structured Wasm LIR (Stackifier)
WebAssembly enforces structured control flow nesting rather than arbitrary jump targets:
1. **Loop Identification:** Tarjan's SCC algorithm discovers natural loops and loop headers.
2. **Block Scheduling:** Reverse Post-Order (RPO) ordering linearizes basic blocks.
3. **Structured Construct Wrapping:**
   - Loop headers are wrapped in Wasm `loop` constructs.
   - Forward edges and merges are enclosed in Wasm `block` constructs.
   - Targets are addressed via relative branch nesting depths (`br <depth>`, `br_if <depth>`).
   - Phi nodes are deconstructed into parallel predecessor copies before structured lowering.

---

## 4. Linear Memory Architecture

The 32-bit linear address space is statically partitioned by `wasmc-memory`:

```
0x0000_0000 ┌────────────────────────────────────────┐
            │ Reserved Guard Region (1024 bytes)     │ Traps null pointer accesses
0x0000_0400 ├────────────────────────────────────────┤
            │ Static Data & String Constants         │ Read-only string literals, globals
            ├────────────────────────────────────────┤
            │ Shadow Stack Limit                     │ Stack overflow trap boundary
            │       ▲                                │
            │       │ (Grows downward)               │ Address-taken locals, spill slots
0x0010_0400 ├───────┴────────────────────────────────┤ (Default: 1MB shadow stack)
            │ __heap_base / Initial __stack_pointer  │
            │       │                                │
            │       ▼ (Grows upward)                 │
            │ Dynamic Heap Memory                    │ TLSF allocator / malloc
0xFFFF_FFFF └────────────────────────────────────────┘
```

- **Global #0 (`__stack_pointer`):** Mutable `i32` global holding the active frame top.
- **Function Prologue:** For functions with address-taken locals, `__stack_pointer` is decremented by the aligned frame size. If stack checking is enabled, `__stack_pointer < stack_limit` traps immediately.
- **Function Epilogue:** `__stack_pointer` is restored to the caller's frame on all exit paths.

---

## 5. Direct Binary Emitter & Section Serialization

WebAssembly Core 2.0 sections are serialized directly into raw bytes obeying strict section ID ordering:

1. `Type` (Section 1)
2. `Import` (Section 2)
3. `Function` (Section 3)
4. `Table` (Section 4)
5. `Memory` (Section 5)
6. `Global` (Section 6)
7. `Export` (Section 7)
8. `Start` (Section 8)
9. `Element` (Section 9)
10. `DataCount` (Section 12, when emitted)
11. `Code` (Section 10)
12. `Data` (Section 11)
13. `Custom` (Section 0, name section & debug info)

Variable-length integers are encoded via a canonical LEB128 codec enforcing 5-byte maximum boundaries for 32-bit values.
