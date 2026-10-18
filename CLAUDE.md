# wasmc — Workspace Constitution
> Direct-to-WebAssembly Systems Compiler · Rust 2024 Edition

## Cargo Workspace

**Root:** `CDD/Cargo.toml` (Cargo workspace, resolver = "2", edition = 2024)
**Members:** 16 crates under `CDD/crates/`
**Binary:** `CDD/target/debug/wasmc.exe` (or `release/wasmc.exe`)

---

## 5-Stage Compiler Architecture

```
Source (.src)
    │
    ▼  Stage 1 — Diagnostics Infrastructure
    │  wasmc-diagnostics   SourceSpan, ErrorCode (E1001–E5002), ANSI/JSON sink
    │
    ▼  Stage 2 — Frontend  (Source → AST)
    │  wasmc-lexer         Zero-copy UTF-8 lexer, SymbolInterner (u32)
    │  wasmc-ast           Arena AstPool, NodeIndex(u32), flat span arrays
    │  wasmc-parser        Recursive descent declarations + Pratt expressions
    │  wasmc-semantics     Bidirectional typechecker, escape analysis,
    │                      StorageClass::VirtualLocal | ShadowStack
    │
    ▼  Stage 3 — Middle-End  (AST → SSA MIR → Optimized MIR)
    │  wasmc-hir           Desugars for/while, compound ops, short-circuit &&/||
    │  wasmc-mir           Braun et al. on-the-fly SSA, BasicBlock CFG, VReg, Phis
    │  wasmc-optimizer     Const fold/prop, algebraic simplification, CSE, DCE
    │
    ▼  Stage 4 — Backend  (MIR → Wasm binary)
    │  wasmc-stackifier    Tarjan SCC loops, RPO scheduling, block/loop/br_if emit
    │  wasmc-memory        Linear memory planner: guard(1 KiB), statics,
    │                      shadow stack (1 MiB default), heap
    │  wasmc-wasm          LEB128 codec, section framing (1–11 + §0), WAT printer
    │  wasmc-bindings      ES6 JS glue + TypeScript .d.ts, WASI preview-1 shim
    │
    ▼  Stage 5 — Tooling  (Pipeline orchestration & services)
       wasmc-cache         SQLite 3 WAL cache, BLAKE3 content hashing, invalidation
       wasmc-driver        End-to-end pipeline coordinator, file I/O, artifact emit
       wasmc-daemon        Axum HTTP/2 REST server — /api/v1/{compile,check,health,cache/status}
       wasmc-cli           Native binary: wasmc build | check | daemon
```

---

## Key Invariants

| Invariant | Detail |
|:---|:---|
| **No LLVM** | Zero dependency on LLVM, Binaryen, Emscripten, or external assemblers |
| **No unsafe Rust** | All 16 crates compile with `#![forbid(unsafe_code)]` |
| **Structured control flow** | Wasm `block`/`loop`/`br`/`br_if` only — no synthetic dispatch variables |
| **Golden artifacts** | `CDD/examples/*.src` and `CDD/examples/*.wat` are committed reference fixtures — never delete them |
| **Cache files excluded** | `*.db`, `*.db-wal`, `*.db-shm`, `.wasmc-cache/` are git-ignored |

---

## Verification Gate Commands

Run these three commands before every commit. All must pass with zero errors and zero warnings.

### 1. Rust Unit & Integration Tests
```bash
cargo test --manifest-path CDD/Cargo.toml --workspace
```
Expected: All 24+ tests pass across all 16 crates.

### 2. Clippy Lint Gate (warnings-as-errors)
```bash
cargo clippy --manifest-path CDD/Cargo.toml --workspace -- -D warnings
```
Expected: Zero warnings, zero errors.

### 3. End-to-End Node.js Dry-Run Suite
```bash
node CDD/scripts/complete_dry_run_test.js
```
Expected: `37/37 TESTS PASSED`

---

## Repository Layout

```
d:\projects\CD\
├── .gitignore                  ← Root ignore (covers CDD/target/, *.db*)
├── CLAUDE.md                   ← This file — workspace constitution
├── AGENTS.md                   ← Agent governance catalog (68 agents, 122 rules)
├── wasmc-final-report.docx     ← Project final report
└── CDD/                        ← wasmc Cargo workspace root
    ├── Cargo.toml / Cargo.lock
    ├── .gitignore              ← Workspace-scoped ignore
    ├── README.md               ← Quickstart and feature overview
    ├── demo.html               ← Standalone browser compiler studio (self-contained)
    ├── rust-toolchain.toml     ← Pinned Rust toolchain
    ├── crates/                 ← 16 compiler crates (see architecture above)
    ├── docs/
    │   ├── ARCHITECTURE.md
    │   ├── LANGUAGE_REFERENCE.md
    │   ├── IMPLEMENTATION_STATUS.md
    │   ├── IMPLEMENTATION_GAP_ANALYSIS.md
    │   ├── TEST_REPORT.md
    │   ├── LIMITATIONS.md
    │   └── specs/              ← Formal IEEE/ISO engineering specifications
    │       ├── srs.md          ← Software Requirements Specification
    │       ├── frd.md          ← Functional Requirements Document
    │       ├── hld.md          ← High-Level Design
    │       ├── lld.md          ← Low-Level Design
    │       └── sdd.md          ← Software Design Document
    ├── examples/               ← Golden test programs (.src, .wat, .wasm, .js, .d.ts)
    └── scripts/
        ├── complete_dry_run_test.js   ← 37-test automated regression suite
        ├── run_wasm.js                ← Universal CLI Wasm runner
        └── build_demo_html.js         ← Regenerates demo.html
```

---

## CLI Quick Reference

```bash
# Compile (from CDD/)
.\target\debug\wasmc.exe build examples/sorting.src -o examples/sorting.wasm --emit-wat

# Semantic check only
.\target\debug\wasmc.exe check examples/sorting.src

# Start REST compilation daemon
.\target\debug\wasmc.exe daemon --host 127.0.0.1 --port 8089

# Run a compiled binary
node scripts/run_wasm.js examples/sorting.wasm

# Open interactive browser studio
start demo.html
```
