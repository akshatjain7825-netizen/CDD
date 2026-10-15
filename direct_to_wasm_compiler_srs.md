# Software Requirements Specification (SRS)

## Project: Custom Direct-to-WebAssembly (Wasm) Compiler Toolchain

**Document Version:** 1.0.0

**Status:** Approved for Implementation

**Target Architecture:** WebAssembly Core Specification (1.0 / 2.0) & WASI Preview 1/2

## 1. Introduction

### 1.1 Purpose

This Software Requirements Specification (SRS) defines the functional, non-functional, and architectural requirements for building a custom, production-grade Direct-to-WebAssembly (Wasm) compiler from scratch. It translates the architectural blueprint and prompt engineering analysis into formal engineering specifications for compiler engineers and systems architects.

### 1.2 Scope

The compiler toolchain ingests a statically typed, imperative/expression-oriented source language and compiles it directly into a valid, optimized WebAssembly binary module (`.wasm`) and optional WebAssembly Text format (`.wat`) without relying on heavy third-party backend frameworks (such as LLVM or Binaryen). The scope encompasses:

* Lexical and syntactic analysis.

* Semantic validation and symbol resolution.

* Multi-level Intermediate Representation (IR) transformation and optimization.

* Stackification and structured control flow recovery.

* Direct binary encoding (LEB128, IEEE 754, Section framing).

* Linear memory layout, shadow stack allocation, and garbage collection/allocator integration.

* Host environment interfacing (JavaScript/Browser ABI and WASI).

### 1.3 Definitions, Acronyms, and Abbreviations

* **ABI:** Application Binary Interface.

* **AST:** Abstract Syntax Tree.

* **CFG:** Control Flow Graph.

* **CSA:** Cytron’s Static Single Assignment construction algorithm.

* **DCE:** Dead Code Elimination.

* **HIR:** High-Level Intermediate Representation.

* **LEB128:** Little Endian Base 128 variable-length encoding.

* **LIR:** Low-Level Intermediate Representation (Stack-scheduled).

* **MIR:** Mid-Level Intermediate Representation (SSA CFG).

* **MVP:** Minimum Viable Product (WebAssembly 1.0 Baseline).

* **RPO:** Reverse Post-Order graph traversal.

* **SSA:** Static Single Assignment.

* **TLSF:** Two-Level Segregate Fit memory allocator.

* **WASI:** WebAssembly System Interface.

* **Wasm:** WebAssembly Binary Format.

* **WAT:** WebAssembly Text Format.

## 2. Overall System Architecture

### 2.1 Compilation Pipeline Overview

The system shall operate as a deterministic, multi-stage compilation pipeline:

$$
\text{Source Code} \xrightarrow{\text{Lexer}} \text{Tokens} \xrightarrow{\text{Parser}} \text{AST (HIR)} \xrightarrow{\text{Typechecker}} \text{Typed HIR} \xrightarrow{\text{Lowering}} \text{SSA MIR} \xrightarrow{\text{Optimizer}} \text{Opt MIR} \xrightarrow{\text{Stackifier}} \text{Wasm LIR} \xrightarrow{\text{Emitter}} \text{.wasm Binary}
$$

### 2.2 Implementation Constraints & Tooling Justification

* **SYS-ARCH-001 (Zero-LLVM Mandate):** The compiler shall emit Wasm bytecode directly. Skipping LLVM eliminates massive dependency footprints, prevents C/C++ ABI impedance mismatches, and enables sub-100ms compilation times suitable for in-browser or edge compilation.

* **SYS-ARCH-002 (Memory Efficiency):** All compiler data structures (AST nodes, IR instructions, Symbol Tables) shall utilize arena allocation or flattened array pools (`uint32_t` indices instead of raw memory pointers) to maximize CPU cache locality and minimize heap fragmentation during compilation.

## 3. Front-End Requirements

### 3.1 Lexical Analysis (Lexer)

* **FE-LEX-001 (Architecture):** The lexer shall be implemented as a hand-written, zero-allocation deterministic finite automaton (DFA) operating over a contiguous UTF-8 byte buffer.

* **FE-LEX-002 (Zero-Copy Tokenization):** Tokens shall not allocate heap strings for identifiers or literals. Each token shall store a byte-offset span `(start_offset: u32, length: u32)` pointing directly into the immutable source buffer.

* **FE-LEX-003 (String Interning):** Identifier strings and string literals shall be deduplicated via a global `StringInterner` backed by a bump-allocated arena and an open-addressed hash table, yielding a lightweight 32-bit `SymbolId`.

* **FE-LEX-004 (Lookahead & Buffering):** The lexer shall provide a $k=1$ (with optional $k=2$) token lookahead buffer to drive predictive parsing without backtracking.

* **FE-LEX-005 (Lexical Error Recovery):** Invalid UTF-8 sequences, unterminated string literals, or unrecognized byte sequences shall emit a structured `LexError` token containing the exact source span and immediately advance to the next synchronizing delimiter (whitespace or newline) without halting the pipeline.

### 3.2 Syntactic Analysis (Parser)

* **FE-PAR-001 (Grammar Architecture):** The parser shall use a hybrid **Recursive Descent** architecture for top-level declarations and control-flow statements, combined with **Pratt Parsing (Top-Down Operator Precedence)** for expressions.

* **FE-PAR-002 (Precedence Binding):** The Pratt parser shall dynamically resolve prefix, infix, and postfix operators using explicit left and right binding power tables ($BP_L, BP_R$), cleanly handling associativity and unary/binary operator overloading (e.g., `-` negation vs. subtraction).

* **FE-PAR-003 (Error Recovery Strategy):** The parser shall implement **Panic-Mode Recovery** fortified with **Synchronizing Token Sets** (e.g., `;`, `}`, `fn`, `let`, `return`). Upon encountering a syntax error, the parser shall:

  1. Record a diagnostic message with precise `(line, column, byte_offset)` coordinates.

  2. Discard incoming tokens until a token in the active synchronizing set is encountered.

  3. Inject a synthetic `PoisonExpr` or `ErrorStmt` node into the AST to maintain structural integrity for downstream semantic analysis.

### 3.3 Abstract Syntax Tree (AST) Design

* **FE-AST-001 (Flattened Arena Topology):** AST nodes shall be stored contiguously in a single `Vec<AstNode>` array rather than as heap-allocated tree pointers. Child nodes shall be referenced via 32-bit integer indices (`NodeIndex`).

* **FE-AST-002 (Struct-of-Arrays Metadata):** Source spans (`Span`) and resolved types (`TypeId`) shall be stored in parallel arrays indexed by `NodeIndex` so that passes that do not require source coordinates (e.g., type checking) do not pollute CPU cache lines with debugging metadata.

### 3.4 Semantic Analysis & Type Checking

* **FE-SEM-001 (Lexical Scoping):** The symbol table shall be implemented as a cactus stack (spaghetti stack) or flat vector of scope frames, where each scope maps `SymbolId -> SymbolEntry` and maintains a parent scope index.

* **FE-SEM-002 (Bidirectional Type Checking):** The type checker shall interleave type synthesis ($\Gamma \vdash e \Rightarrow \tau$, bottom-up inference) and type checking ($\Gamma \vdash e \Leftarrow \tau$, top-down verification) to infer local variable types while enforcing explicit function boundaries.

* **FE-SEM-003 (Poison Type Propagation):** Any expression referencing an `ErrorStmt` or failing type unification shall be assigned a special $\tau_{\text{error}}$ (`PoisonType`). Unification rules shall dictate that $\tau_{\text{error}}$ unifies silently with any type $\tau$, suppressing cascading secondary type errors.

* **FE-SEM-004 (Wasm Semantic Pre-Validation):** The semantic pass shall verify properties required by the Wasm backend:

  * Strict differentiation between 32-bit and 64-bit integer/float operations (no implicit widening casts without inserting explicit AST `CastExpr` nodes).

  * Validation that all non-`void` control flow paths return a value matching the function signature.

  * Identification of variables whose addresses are taken (`&x`), flagging them as `EscapedToLinearMemory` so they are allocated on the shadow stack rather than in Wasm virtual registers (`local.get`/`local.set`).

## 4. Intermediate Representation (IR) & Optimization Pipeline

### 4.1 Multi-Tier IR Architecture

The compiler shall implement three distinct IR stages to bridge the semantic gap between high-level source code and WebAssembly’s structured stack machine:

1. **High-Level IR (HIR):** A desugared, strictly typed AST where syntactic sugar (`for` loops, compound assignments `+=`, short-circuiting boolean operators) is lowered into canonical forms, and generics/templates are monomorphized.

2. **Mid-Level IR (MIR - SSA CFG):** A flattening of the HIR into basic blocks connected via explicit control-flow edges, utilizing Static Single Assignment (SSA) form where every virtual register is assigned exactly once.

3. **Low-Level IR (LIR - Wasm Structured Stack IR):** A linear sequence of instructions grouped into nested Wasm structured blocks (`block`, `loop`, `if`), where SSA virtual registers are converted into either implicit value-stack transfers or indexed Wasm locals.

### 4.2 MIR (SSA) Specification

* **IR-MIR-001 (SSA Construction):** The compiler shall construct pruned SSA form using **Braun et al.’s algorithm** (or Cytron’s dominance frontier algorithm), placing $\phi$ (phi) nodes at control-flow join points.

* **IR-MIR-002 (Three-Address Instructions):** MIR instructions shall operate on typed virtual registers (`%v0: i32`, `%v1: f64`). Memory accesses shall explicitly encode static alignment and static byte offsets (`load.i32 %base, offset=8, align=4`).

* **IR-MIR-003 (Control Flow Graph):** Each basic block shall terminate with an explicit control instruction: `Jump(BlockId)`, `Branch(CondReg, TrueBlockId, FalseBlockId)`, `Switch(ValReg, DefaultBlockId, [TargetTable])`, `Return(OptReg)`, or `Trap`.

### 4.3 Optimization Passes

The MIR optimizer shall execute a configurable pipeline of deterministic passes:

* **IR-OPT-001 (Constant Folding & Propagation):** Evaluate compile-time constant expressions (using strict Wasm IEEE 754 and two's complement wrap-around semantics) and substitute constant virtual registers across the CFG.

* **IR-OPT-002 (Algebraic Simplification & Strength Reduction):**

  * Transform $x \times 2^n \rightarrow x \ll n$.

  * Transform unsigned division $x / 2^n \rightarrow x \gg_u n$.

  * Eliminate identity operations ($x + 0 \rightarrow x$, $x \land \text{0xFFFFFFFF} \rightarrow x$).

* **IR-OPT-003 (Common Subexpression Elimination - CSE):** Utilize Dominator-tree Value Numbering (DVNT) to eliminate redundant pure computations within dominating basic blocks.

* **IR-OPT-004 (Memory-to-Register Promotion - `Mem2Reg`):** Promote non-escaping local variables from linear memory stack slots into SSA virtual registers.

* **IR-OPT-005 (Dead Code Elimination - DCE):** Remove unused SSA definitions via mark-and-sweep over use-def chains, and prune unreachable basic blocks discovered during CFG traversal.

* **IR-OPT-006 (Inlining):** Inline small leaf functions whose instruction count falls below a configurable threshold ($N \le 25$ MIR instructions) and have no recursive call edges.

## 5. Code Generation Strategy (Target: WebAssembly Binary Format)

### 5.1 Structured Control Flow Recovery (CFG to Wasm Blocks)

Because WebAssembly forbids arbitrary `goto` jumps, the backend must translate the unstructured/reducible MIR CFG into Wasm’s structured control instructions (`block`, `loop`, `if`, `br`, `br_if`, `br_table`).

* **CG-SCF-001 (Reducibility Enforcement):** Prior to lowering, the compiler shall verify that the CFG is reducible (all loops are natural loops with a single entry header dominating all nodes in the loop). If irreducible control flow exists, the compiler shall apply node splitting or dispatcher transformation to restore reducibility.

* **CG-SCF-002 (Stackifier / Relooper Lowering):** The backend shall implement a **Stackifier** algorithm (based on dominance tree and Reverse Post-Order traversal):

  * Compute the dominance tree and loop nesting forest of the MIR CFG.

  * Order basic blocks in Reverse Post-Order (RPO), ensuring loop bodies remain contiguous immediately following their loop headers.

  * Insert `loop ... end` scopes around backward branch targets (where the target block precedes the source block in RPO).

  * Insert `block ... end` scopes around forward branch merge points (where the target block follows the source block in RPO).

  * Compute relative label depths ($d \in \mathbb{N}_0$) for `br $d` and `br_if $d` instructions by counting the number of intervening enclosing `block`/`loop`/`if` frames on the control stack.

### 5.2 Stack Machine Mapping & Local Allocation

* **CG-STK-001 (Expression Tree Stackification):** For single-use SSA variables whose definition and consumption occur within the same basic block without intervening side-effecting instructions (memory stores, function calls, global mutations), the compiler shall elide `local.set` and `local.get` entirely, leaving the value directly on the Wasm operand stack.

* **CG-STK-002 (Tee Peephole Optimization):** Where a value is pushed onto the stack, written to a local, and immediately consumed by the next instruction (`local.set $x` followed immediately by `local.get $x`), the emitter shall fuse the sequence into a single `local.tee $x` instruction.

* **CG-STK-003 (SSA Deconstruction & Register Coalescing):** Multi-use or cross-block SSA virtual registers and $\phi$-nodes shall be lowered to Wasm local variables (`local.get`/`local.set`). The backend shall apply **Linear Scan Register Allocation with Liveness Interference Coloring** to map non-overlapping SSA lifetimes onto the minimum possible number of typed Wasm local indices (`i32`, `i64`, `f32`, `f64`).

### 5.3 Binary Encoding Engine

The compiler shall emit a compliant binary stream starting with the preamble magic number `\0asm` (`0x00 0x61 0x73 0x6D`) and version `1` (`0x01 0x00 0x00 0x00`).

* **CG-BIN-001 (LEB128 Encoding):** All variable-length integer immediates, section byte sizes, vector lengths, and local/function indices shall be encoded using unsigned (`uLEB128`) or two's complement signed (`sLEB128`) encodings.

* **CG-BIN-002 (Floating-Point Encoding):** `f32.const` and `f64.const` immediates shall be emitted in little-endian IEEE 754-2008 binary format, preserving exact bit patterns for canonical and payload `NaN` values.

* **CG-BIN-003 (Strict Section Ordering):** The binary emitter shall serialize known sections in strict ascending numerical order by Section ID. Empty sections shall be omitted entirely. Each section shall be framed as `SectionID (1 byte) | ByteLength (uLEB128) | Payload`:

| ID | Section Name | Responsibility & Contents | 
| ----- | ----- | ----- | 
| `1` | **Type** | Deduplicated vector of canonical function signatures `(func (param ...) (result ...))`. | 
| `2` | **Import** | External host functions, memories, tables, and globals declared via module/field UTF-8 strings. | 
| `3` | **Function** | Array mapping internal function indices to their corresponding signature index in the Type section. | 
| `4` | **Table** | Indirect call table declarations (`funcref` / `externref`) for dynamic dispatch and function pointers. | 
| `5` | **Memory** | Linear memory bounds declarations (`min` pages, optional `max` pages, where $1\text{ page} = 64\text{ KiB}$). | 
| `6` | **Global** | Module-scoped mutable and immutable global variables (including the `__stack_pointer` register). | 
| `7` | **Export** | Public interface table exposing internal functions, memory (`"memory"`), or globals to the host. | 
| `8` | **Start** | Optional module initializer function index executed automatically upon instantiation. | 
| `9` | **Element** | Static initialization segments populating the `funcref` Table for indirect calls (`call_indirect`). | 
| `12` | **DataCount** | Count of data segments (required if emitting bulk memory operations like `memory.init`). | 
| `10` | **Code** | Compressed local variable declarations and instruction bytecode streams (terminated by `0x0B`) for each internal function. | 
| `11` | **Data** | Static byte arrays (string literals, global structs) initialized into linear memory at compile-time offsets. | 
| `0` | **Custom** | Appended last; includes the standard `"name"` section (function/local debug symbols) and DWARF/SourceMap sections. | 

## 6. Memory Management & Linear Memory Layout

### 6.1 Linear Memory Topology

Because Wasm’s native call stack is completely opaque and immune to pointer arithmetic, the compiler shall partition the 32-bit byte-addressable linear memory space (`wasm32`) into four strict regions:

```
+-----------------------------------------------------------------------+
| 0x00000000          | 0x00000400        | __data_end     | __heap_base|
| Unmapped Null Guard | Static Data (.ro) | Shadow Stack   | Dynamic    |
| Zone (1024 Bytes)   | & Globals (.bss)  | (Grows Down v) | Heap (^ Up)|
+-----------------------------------------------------------------------+
                                          <-- SP             Heap Ptr -->


```

* **MM-LAY-001 (Null-Pointer Guard Zone):** Bytes `0x00000000` through `0x000003FF` ($1024\text{ bytes}$) shall remain unallocated. Any static data segment placement shall start at or above `0x00000400` so that null pointer (`0x0`) dereferences can be caught via compiler-inserted debug checks or static offset validation.

* **MM-LAY-002 (Static Data & BSS Region):** String literals, constant arrays, and vtables shall be packed into the Data section starting at `0x00000400`, aligned to their natural byte boundaries ($1, 2, 4, \text{ or } 8\text{ bytes}$).

* **MM-LAY-003 (Downward-Growing Shadow Stack):** Located immediately after `__data_end` with a fixed default reservation (configurable, default $1\text{ MiB}$). The top of this region is initialized as the starting value of a mutable Wasm global: `(global $__stack_pointer (mut i32) (i32.const <__heap_base>))`.

* **MM-LAY-004 (Upward-Growing Dynamic Heap):** Starts at `__heap_base` and expands dynamically toward higher memory addresses using the `memory.grow` instruction ($64\text{ KiB}$ page increments).

### 6.2 Calling Convention & Shadow Stack Frame ABI

* **MM-ABI-001 (Scalar Passing):** Scalar primitives (`i32`, `i64`, `f32`, `f64`) and raw linear memory pointers (`i32`) shall be passed directly via Wasm parameters and returned via Wasm multi-value or scalar return registers.

* **MM-ABI-002 (Aggregate & Escaped Variable Passing):** Structs, arrays, unions, and local primitives whose addresses are taken (`&x`) shall be allocated on the linear memory **Shadow Stack**.

* **MM-ABI-003 (Stack Frame Prologue/Epilogue):** Functions requiring a shadow stack frame of $K$ bytes (aligned to a 16-byte boundary) shall emit:

  * **Prologue:** Fetch `global.get $__stack_pointer`, subtract $K$, store the new frame pointer in a local `$fp`, and update `global.set $__stack_pointer`.

  * **Body:** Access frame variables via static offsets (`i32.load offset=N (local.get $fp)`).

  * **Epilogue:** Before every `return` instruction, restore `global.set $__stack_pointer (i32.add (local.get $fp) (i32.const K))`.

### 6.3 Heap Allocation & Garbage Collection Strategy

* **MM-ALC-001 (Embedded Runtime Allocator):** The compiler shall bundle a lightweight, freestanding allocator compiled directly into the Wasm module:

  * **Mode A (Manual / Arena / Rust-style Ownership):** Embed a **Two-Level Segregate Fit (TLSF)** or `dlmalloc`-style free-list allocator (\~2 KiB code footprint) providing $O(1)$ `malloc` and `free` backed by `memory.grow`.

  * **Mode B (Managed / Garbage Collected):** If the source language requires automatic memory management on baseline Wasm 1.0/2.0 engines, embed a non-moving mark-and-sweep collector. The compiler shall emit **Shadow Stack Roots** by registering live pointer-containing locals into the linear memory shadow stack prior to any function call that may trigger a GC cycle.

## 7. Runtime Integration & Host Environment Interfacing

### 7.1 Dual-Target Host ABI

The compiler shall support a `--target` flag allowing developers to target two primary runtime environments:

1. `wasm32-unknown-web`: Browser and JavaScript runtimes (V8, SpiderMonkey, JavaScriptCore, Node.js, Deno).

2. `wasm32-wasi`: Standalone system runtimes (Wasmtime, WasmEdge, Wasmer) adhering to the WebAssembly System Interface (WASI).

### 7.2 JavaScript / Browser Host Bindings (`wasm32-unknown-web`)

* **RT-JS-001 (Auto-Generated JS Glue Code):** When targeting the web, the compiler shall optionally output an ES6 Module wrapper (`<module>.js`) and TypeScript definitions (`<module>.d.ts`) alongside the `.wasm` binary.

* **RT-JS-002 (String & Array Marshalling Across the Boundary):** Because Wasm functions only accept numeric primitives across the boundary:

  * **Host** $\rightarrow$ **Wasm:** The JS glue code shall encode JS strings into UTF-8 via `TextEncoder`, invoke the Wasm exported `__alloc(byte_len, align)` function, copy bytes into `new Uint8Array(wasmMemory.buffer, ptr, byte_len)`, and pass `(ptr: i32, byte_len: i32)` into the target Wasm function.

  * **Wasm** $\rightarrow$ **Host:** Wasm shall return a packed `i64` (upper 32 bits = pointer, lower 32 bits = length) or write a `(ptr, len)` tuple to a return slot; the JS glue code shall decode the slice using `TextDecoder.decode(new Uint8Array(wasmMemory.buffer, ptr, len))` and invoke `__dealloc(ptr, len)`.

* **RT-JS-003 (DOM & Host Object References):** For passing opaque JS objects (`Window`, `CanvasRenderingContext2D`, closures) into Wasm:

  * If `--enable-reference-types` is active, utilize native Wasm `externref` tables.

  * Otherwise, maintain a slab-allocated `HeapObjectTable` array in the JS glue layer and pass 32-bit integer handle indices across the Wasm boundary.

### 7.3 WASI Integration (`wasm32-wasi`)

* **RT-WASI-001 (Entrypoint Signature):** In WASI command mode, the compiler shall synthesize and export `_start: () -> ()`, which initializes the shadow stack pointer, runs static constructors, invokes the user’s `main()` function, and passes the exit code to `wasi_snapshot_preview1.proc_exit`.

* **RT-WASI-002 (System Call Lowering):** Standard library I/O operations (e.g., `print`, file read/write, clock queries, random number generation) shall be lowered directly to WASI imports (`fd_write`, `fd_read`, `clock_time_get`, `random_get`) using `ciovec` scatter/gather structs allocated on the shadow stack.

### 7.4 Trap Handling & Diagnostics

* **RT-TRP-001 (Hardware/Engine Traps):** Operations that violate Wasm execution semantics—such as integer division by zero (`i32.div_s`), unrepresentable float-to-int conversions (`i32.trunc_f64_s` on overflow/NaN), out-of-bounds linear memory accesses, and explicit `unreachable` (`0x00`) instructions—will trigger an immediate host `WebAssembly.RuntimeError`.

* **RT-TRP-002 (Saturating Casts):** By default, float-to-integer casts shall emit non-trapping saturating instructions (`i32.trunc_sat_f64_s`, opcode prefix `0xFC`) to prevent unexpected runtime crashes.

* **RT-TRP-003 (Custom Panic Hook):** In debug builds (`--debug`), the compiler shall wrap division operations, array bounds checks, and shadow stack overflow checks (`$__stack_pointer < $__data_end`) with explicit branches to a `@__wasm_panic(msg_ptr: i32, msg_len: i32, file_ptr: i32, line: i32)` intrinsic. This intrinsic invokes an imported host logger before executing the `unreachable` opcode.

* **RT-TRP-004 (Custom `"name"` Section & Source Maps):** In debug or profiling builds, the binary emitter shall append the WebAssembly `"name"` Custom Section (mapping function indices and local indices to human-readable identifiers) so browser DevTools and standalone runtimes display exact source function names in stack traces.

## 8. Non-Functional Requirements & Verification

### 8.1 Performance & Resource Constraints

* **NFR-PERF-001 (Compilation Throughput):** The compiler shall process and emit valid Wasm binaries at a minimum throughput of $50,000$ lines of source code per second on a standard single-core modern CPU.

* **NFR-PERF-002 (Binary Footprint):** A minimal "Hello World" program targeting `wasm32-unknown-web` with dead-code elimination enabled shall produce a `.wasm` binary smaller than $512\text{ bytes}$.

* **NFR-PERF-003 (Deterministic Reproducibility):** Given identical source files and compiler flags, the compiler shall emit a bit-for-bit identical `.wasm` binary across all host operating systems (Linux, macOS, Windows, and self-hosted Wasm).

### 8.2 Verification & Validation Matrix

Every stage of the compiler pipeline shall be validated via automated test harnesses:

1. **Bytecode Spec Validation:** 100% of emitted `.wasm` binaries in the test suite must pass strict structural and stack-type validation using the official WebAssembly Binary Toolkit (`wasm-validate` from WABT) and `wasm-tools validate`.

2. **Differential Execution Testing:** Compiler outputs for arithmetic, control flow, and memory benchmarks shall be executed inside both **Wasmtime** (AOT/JIT) and **Node.js/V8**, verifying identical stdout/return values across optimization levels (`-O0` vs. `-O3`).

3. **Fuzzing Resilience:** The Lexer, Parser, and Stackifier shall undergo continuous coverage-guided fuzzing to verify that malformed inputs never cause compiler panics, infinite loops, or invalid Wasm stack height mismatches.