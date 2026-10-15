# `wasmc` — Direct-to-WebAssembly Compiler

[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-orange.svg)](https://www.rust-lang.org/)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-Core%202.0-blue.svg)](https://webassembly.org/)
[![License](https://img.shields.io/badge/License-Apache%202.0%20%2F%20MIT-green.svg)](LICENSE)

`wasmc` is a high-performance, optimizing direct-to-binary WebAssembly compiler toolchain written in Rust (2024 edition).

It translates a statically typed, imperative systems programming language directly into valid WebAssembly binary modules (`.wasm`) **without routing compilation through LLVM, Binaryen, C/C++, Emscripten, or external assemblers**.

---

## Key Features

- **Direct Binary Emission:** Emits WebAssembly Core 2.0 binaries directly with bounded LEB128 encoding and strict section framing (Sections 1..11 + custom 0).
- **SSA MIR & Optimizations:** Constructs Static Single Assignment (SSA) form on-the-fly via the Braun et al. algorithm. Performs constant folding/propagation, algebraic simplifications, strength reduction, Common Subexpression Elimination (CSE), and Dead-Code Elimination (DCE).
- **Tarjan Stackifier:** Converts arbitrary control-flow graphs into structured WebAssembly blocks (`block`, `loop`, `if`, and relative `br`/`br_if` depths) with zero synthetic dispatch variables.
- **Deterministic Linear Memory Model:** Features a 1024-byte null-pointer guard region, static data, a downward-growing shadow stack for address-taken locals, and an upward-growing heap.
- **Incremental Compilation Cache:** Built-in SQLite 3 WAL-mode content-addressable cache keyed by BLAKE3 hashes.
- **Multi-Target Support:** Emits browser ES6 JS glue and TypeScript `.d.ts` declarations (`wasm32-unknown-web`) or WASI preview 1 runner shims (`wasm32-wasi`).
- **Compiler Daemon:** Embedded Axum HTTP/2 REST service with `/api/v1/compile`, `/api/v1/check`, and `/api/v1/cache/status` endpoints.

---

## Workspace Architecture

```
crates/
├── wasmc-diagnostics   # Error codes, spans, and ANSI/JSON diagnostic sink
├── wasmc-lexer         # Zero-copy DFA lexer & 32-bit symbol interner
├── wasmc-ast           # Flattened arena AST pool (NodeIndex(u32))
├── wasmc-parser        # Hybrid Recursive Descent + Pratt expression parser
├── wasmc-semantics     # Scopes, bidirectional typechecker, escape analysis
├── wasmc-hir           # High-Level IR canonicalization and loop desugaring
├── wasmc-mir           # Braun et al. on-the-fly SSA CFG representation
├── wasmc-optimizer     # Multi-pass deterministic optimization pipeline
├── wasmc-stackifier    # Tarjan SCC loop analysis and structured block lowerer
├── wasmc-memory        # Memory planner, shadow stack, and heap layout
├── wasmc-wasm          # LEB128 codec, section serializer, and WAT printer
├── wasmc-bindings      # Browser JS/TS and WASI runtime glue generators
├── wasmc-cache         # SQLite 3 WAL content-addressable build cache
├── wasmc-driver        # End-to-end compilation pipeline driver
├── wasmc-daemon        # Axum HTTP/2 REST compilation server
└── wasmc-cli           # Native CLI executable (`wasmc`)
```

---

## Quickstart & CLI Usage

### Build the Toolchain
```bash
cargo build --workspace --release
```

### 1. Compile Source Directly to WebAssembly
```bash
wasmc build examples/arithmetic.src -o arithmetic.wasm --emit-wat
```

### 2. Validate Source Code (`check`)
```bash
wasmc check examples/control_flow.src
```

### 3. Start the Compiler Daemon
```bash
wasmc daemon --host 127.0.0.1 --port 8080
```

---

## Source Language Example

```src
// Function export with primitive types
export fn sum_up_to(n: i32) -> i32 {
    let mut sum: i32 = 0;
    let mut i: i32 = 1;
    while (i <= n) {
        sum = sum + i;
        i = i + 1;
    }
    return sum;
}

// Host import and string literals
import "env" "print" fn print(ptr: *char, len: i32) -> void;

export fn main() -> i32 {
    let msg: *char = "Hello, WebAssembly!";
    print(msg, 19);
    return 0;
}
```

---

## Running Compiled WebAssembly in Node.js

```javascript
const fs = require('fs');
const bytes = fs.readFileSync('arithmetic.wasm');

WebAssembly.instantiate(bytes).then(({ instance }) => {
  const { add, compute, float_calc } = instance.exports;
  console.log('add(40, 2) =', add(40, 2));             // 42
  console.log('compute(10, 20) =', compute(10, 20));   // 205
  console.log('float_calc(2.0, 3.0) =', float_calc(2.0, 3.0)); // 7
});
```

---

## Running the Test Suite

```bash
cargo test --workspace
```

---

## Technical Documentation

- [Architecture Specification](docs/ARCHITECTURE.md)
- [Language Reference Manual](docs/LANGUAGE_REFERENCE.md)
- [Implementation Gap Analysis](docs/IMPLEMENTATION_GAP_ANALYSIS.md)
- [Requirement Traceability Matrix](docs/IMPLEMENTATION_STATUS.md)
- [Automated Test & Verification Report](docs/TEST_REPORT.md)
- [Toolchain Limitations](docs/LIMITATIONS.md)

---

## License

Licensed under either of Apache License, Version 2.0 (`LICENSE-APACHE`) or MIT license (`LICENSE-MIT`).
