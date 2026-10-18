# Toolchain Limitations & Scope Constraints — `wasmc`

> **Living Docs Role: Live Status**
> Governed under ECC `living-docs-governance`. This document defines explicit toolchain boundaries, unsupported proposals, and intentional non-goals.

## 1. Scope & Target Constraints

### 1.1 WebAssembly Specification Profile
- The compiler targets **WebAssembly Core 2.0** (`wasm32-unknown-web` and `wasm32-wasi`).
- Binary modules are emitted directly using 32-bit linear memory (`wasm32`). 64-bit linear memory (`memory64`) is not currently emitted.
- While exception handling opcodes (`try_table`, `throw`) and tail-call opcodes (`return_call`) are supported in the IR and codegen, executing them in production requires host runtimes supporting the respective W3C Phase 4 proposals (`--experimental-wasm-exnref`, `--experimental-wasm-return-call`).

### 1.2 Multi-Module Linking
- The current version of `wasmc` performs single-module translation from one or more source compilation units into a single `.wasm` binary module.
- Static object linking (`wasm-ld` style `.o` object relocation) is not implemented; external functionality is consumed via standard WebAssembly imports (`import "env" "fn"`).

### 1.3 Debugging Information (DWARF v5)
- The compiler generates custom WebAssembly `name` sections providing function and local names for source-level stack traces in browser devtools and Wasm runtimes.
- Full DWARF v5 section embedding (`.debug_info`, `.debug_line`, `.debug_ranges`) is partially specified in the LLD and planned for a future milestone; basic source mapping is achieved via `--emit-wat` and `name` sections.

### 1.4 Garbage Collection & Reference Types
- The primary memory management model implemented is manual memory management with a downward-growing shadow stack and dynamic linear memory heap.
- The W3C Garbage Collection (WasmGC) proposal is not used for primitive language types, ensuring modules can run on any standard Core 2.0 runtime without requiring specialized GC hosts.
