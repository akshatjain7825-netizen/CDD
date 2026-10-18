# Functional Requirements Document (FRD)

## Project: Custom Direct-to-WebAssembly (Wasm) Compiler Toolchain

## Part 1: Pre-FRD Research & Gap Analysis

Before formalizing the functional requirements, a thorough review of the **Direct-to-Wasm Compiler SRS (`wasm_compiler_srs.md`)** revealed several implicit assumptions, architectural boundary areas, and specification gaps that require explicit handling or stakeholder sign-off:

1. **Source Language Grammar Ambiguity:** The SRS specifies supporting composite types (structs, arrays, unions, pointers), generics/templates (Section 4.1), and imperative/expression-oriented constructs, yet does not formally lock down the exact source language syntax (e.g., whether it compiles a custom domain-specific language, a subset of C/Rust, or a bespoke language).

2. **Garbage Collection vs. Manual Memory Management Selection:** Section 6.3 (`MM-ALC-001`) defines two distinct allocation modes—**Mode A** (Manual / Arena / TLSF free-list) and **Mode B** (Managed / Mark-and-Sweep GC with shadow stack roots)—but does not define how the user or source language selects between these modes at compile time.

3. **Multi-Module Linking & Separate Compilation:** While `Import` (ID 2), `Export` (ID 7), `Table` (ID 4), and `Element` (ID 9) sections are specified, the workflow for compiling multiple source files and linking object files versus single-unit whole-program compilation requires explicit scoping.

4. **CLI / Invocation Interface Contract:** The SRS mentions several flags (`--target`, `--debug`, `--enable-reference-types`, `-O0`, `-O3`) across Sections 7 and 8, but omits a unified user-facing invocation contract specifying default behaviors and output artifact paths.

## Part 2: Functional Requirements Specification

### 1. Document Overview

| Attribute | Details | 
| ----- | ----- | 
| **Document Title** | Functional Requirements Document: Direct-to-WebAssembly (Wasm) Compiler | 
| **Version** | 1.0.0 | 
| **Release Date** | October 14, 2026 | 
| **Author** | Lead Business Analyst & Senior Systems Architect | 
| **Status** | Approved for Engineering & QA | 
| **Source Document** | `wasm_compiler_srs.md` (Version 1.0.0) | 

#### 1.1 Purpose

This Functional Requirements Document (FRD) defines the functional behavior, business rules, data interactions, and user workflows required to build the Direct-to-WebAssembly Compiler. It translates the technical specifications of the SRS into testable, requirement-driven deliverables focused on **what** the system must accomplish.

#### 1.2 Scope

* **In Scope:** Source code ingestion, lexical/syntactic validation, bidirectional type checking and escape analysis, multi-tier IR optimization (HIR $\rightarrow$ MIR $\rightarrow$ LIR), structured control-flow transformation (Stackifier), Wasm binary (`.wasm`) serialization, linear memory partitioning, host JavaScript/TypeScript glue generation (`wasm32-unknown-web`), and WASI system call lowering (`wasm32-wasi`).

* **Out of Scope:** Integrated Development Environment (IDE) plugins, third-party package managers, LLVM/Binaryen backend integrations (explicitly prohibited by `SYS-ARCH-001`), and custom host virtual machine execution engines.

#### 1.3 Intended Audience

* **Product Owners & Stakeholders:** To validate business alignment, target use cases, and scope completeness.

* **Compiler & Systems Engineers:** To understand required functional behaviors, inputs, outputs, and system boundaries.

* **Quality Assurance (QA) & Test Engineers:** To derive automated test suites directly from the Given/When/Then acceptance criteria.

### 2. Business Context

#### 2.1 Business Problem

Organizations deploying compute-intensive applications across web browsers, edge networks, and serverless environments face high compilation latency, massive toolchain footprints, and C/C++ ABI impedance mismatches when relying on heavy multi-stage toolchains (such as LLVM or Emscripten). These legacy pipelines prevent sub-100ms compilation times needed for in-browser or edge compilation and introduce significant runtime overhead when marshalling data across the host boundary.

#### 2.2 Business Objectives

1. **Ultra-Fast, Zero-Dependency Compilation:** Eliminate third-party backend dependencies (LLVM/Binaryen) to achieve compilation throughputs exceeding $50,000$ lines of code per second and sub-100ms turnaround times.

2. **Compact Binary Footprint:** Emit canonically encoded, dead-code-stripped `.wasm` binaries where a minimal program targeted for the web requires less than $512\text{ bytes}$.

3. **Dual-Environment Portability:** Enable compiled artifacts to execute seamlessly in both web browsers (with automated JS/TS bindings) and standalone cloud-native/edge runtimes (via WASI).

4. **Deterministic & Verifiable Output:** Guarantee 100% bit-for-bit reproducible binaries across all host operating systems that pass official W3C specification validators (`wasm-validate`).

### 3. User Roles & Personas

| Role / Persona | Description | Primary Responsibilities & System Expectations | 
| ----- | ----- | ----- | 
| **Application Developer** | Software engineer authoring source code targeting Wasm. | Invokes the compiler, resolves syntax/semantic errors via clear multi-error diagnostics, and debugs compiled modules in browser DevTools using custom `"name"` sections and source maps. | 
| **Host Integration Engineer** | Frontend (JS/TS) or Backend/Edge engineer embedding the `.wasm` module. | Consumes auto-generated ES6 `.js` and `.d.ts` wrappers, passes strings/arrays and host object references across the FFI boundary, and supplies custom host imports. | 
| **DevOps / Release Engineer** | Engineer managing CI/CD build pipelines and edge deployments. | Configures build flags (`--target`, `--debug`, optimization levels), verifies deterministic binary hashes across OS environments, and deploys WASI modules to serverless nodes. | 
| **Target Wasm Runtime (System Actor)** | Automated validation and execution host (e.g., V8, SpiderMonkey, Wasmtime). | Validates the 8-byte binary preamble, strict section ordering, LEB128 encodings, and operand stack balance at load time; executes instructions and traps on illegal operations. | 

### 4. Functional Requirements

#### 4.1 Front-End Processing (Lexical, Syntactic, and Semantic Analysis)

##### **FR-001: Zero-Copy Source Tokenization & Interning**

* **Description:** The system shall scan UTF-8 source files into a stream of lexical tokens represented by byte-offset spans `(start_offset, length)` and deduplicate all identifiers and string literals into unique 32-bit symbol identifiers.

* **Priority:** Must Have

* **Source Reference:** SRS Section 3.1 (`FE-LEX-001`, `FE-LEX-002`, `FE-LEX-003`, `FE-LEX-004`)

* **Dependencies:** None

* **Acceptance Criteria:**

  * **Given** a valid UTF-8 source code file containing repeated identifier names and literals,

  * **When** the lexical analyzer processes the input buffer,

  * **Then** the system emits tokens referencing exact byte spans in the source buffer and assigns identical 32-bit `SymbolId` values to identical identifier strings without duplicating heap strings.

##### **FR-002: Non-Halting Lexical Error Recovery**

* **Description:** Upon encountering invalid UTF-8 sequences, unterminated string literals, or unrecognized byte sequences, the system shall emit a structured lexical error token with the exact source span and advance to the next whitespace or newline delimiter to continue scanning.

* **Priority:** Must Have

* **Source Reference:** SRS Section 3.1 (`FE-LEX-005`)

* **Dependencies:** FR-001

* **Acceptance Criteria:**

  * **Given** a source file containing an invalid UTF-8 byte sequence on line 5 and an unterminated string literal on line 20,

  * **When** the lexer scans the file,

  * **Then** the system records both lexical errors with their exact source spans in a single pass without terminating compilation prematurely.

##### **FR-003: Syntactic Parsing & Operator Precedence Resolution**

* **Description:** The system shall parse top-level declarations, control-flow statements, and expressions (resolving prefix, infix, and postfix operator precedence and associativity) to construct a contiguous Abstract Syntax Tree (AST) with parallel source-span metadata.

* **Priority:** Must Have

* **Source Reference:** SRS Section 3.2 (`FE-PAR-001`, `FE-PAR-002`) & Section 3.3 (`FE-AST-001`, `FE-AST-002`)

* **Dependencies:** FR-001

* **Acceptance Criteria:**

  * **Given** a stream of valid tokens containing nested declarations and unparenthesized unary/binary mathematical expressions,

  * **When** the parser executes,

  * **Then** the system constructs a flattened AST array indexed by 32-bit `NodeIndex` values that accurately reflects the defined operator binding powers and stores source coordinates in a parallel array.

##### **FR-004: Panic-Mode Syntax Error Synchronization**

* **Description:** When a syntax error is encountered, the system shall log a diagnostic with `(line, column, byte_offset)`, discard tokens until a synchronizing token (`;`, `}`, `fn`, `let`, `return`) is reached, and inject a synthetic poison/error node into the AST.

* **Priority:** Must Have

* **Source Reference:** SRS Section 3.2 (`FE-PAR-003`)

* **Dependencies:** FR-003

* **Acceptance Criteria:**

  * **Given** a source file with a malformed expression inside a statement followed by a valid statement after a `;` delimiter,

  * **When** the parser processes the token stream,

  * **Then** the system logs the syntax error coordinates, synchronizes at the `;` token, inserts a `PoisonExpr`/`ErrorStmt` node, and successfully parses the subsequent valid statement.

##### **FR-005: Scoped Symbol Resolution & Bidirectional Type Checking**

* **Description:** The system shall resolve identifiers across nested lexical scopes, infer local variable types, enforce explicit function signature types, and suppress cascading secondary type errors by unifying `PoisonType` nodes silently with any type.

* **Priority:** Must Have

* **Source Reference:** SRS Section 3.4 (`FE-SEM-001`, `FE-SEM-002`, `FE-SEM-003`)

* **Dependencies:** FR-003, FR-004

* **Acceptance Criteria:**

  * **Given** an AST containing both valid functions and an expression referencing a syntax `ErrorStmt`,

  * **When** the semantic analyzer performs type checking,

  * **Then** valid local variables have their types inferred, mismatched types trigger diagnostics, and the `ErrorStmt` is assigned `PoisonType` without generating redundant downstream type errors.

##### **FR-006: Wasm Semantic Pre-Validation & Escape Analysis**

* **Description:** The system shall enforce explicit cast nodes between 32-bit and 64-bit numeric types, verify that all non-`void` control paths return a valid value, and flag any local variable whose memory address is taken (`&x`) as escaped to linear memory.

* **Priority:** Must Have

* **Source Reference:** SRS Section 3.4 (`FE-SEM-004`)

* **Dependencies:** FR-005

* **Acceptance Criteria:**

  * **Given** a function containing an implicit 32-to-64-bit assignment, a local variable `x` referenced via `&x`, and a local variable `y` used only by value,

  * **When** semantic pre-validation executes,

  * **Then** an explicit `CastExpr` is required/inserted, variable `x` is flagged as `EscapedToLinearMemory` (for shadow stack allocation), and variable `y` remains eligible for Wasm virtual registers.

#### 4.2 Intermediate Representation (IR) & Optimization Pipeline

##### **FR-007: HIR Desugaring & SSA MIR Construction**

* **Description:** The system shall desugar high-level syntactic constructs and monomorphize generics into a High-Level IR (HIR), then lower the HIR into a Mid-Level IR (MIR) Control Flow Graph in pruned Static Single Assignment (SSA) form with $\phi$-nodes at join points and explicit basic block terminators.

* **Priority:** Must Have

* **Source Reference:** SRS Section 4.1 & Section 4.2 (`IR-MIR-001`, `IR-MIR-002`, `IR-MIR-003`)

* **Dependencies:** FR-006

* **Acceptance Criteria:**

  * **Given** a typed AST containing `for` loops, compound assignments (`+=`), and conditional branches modifying a variable,

  * **When** lowered to MIR,

  * **Then** syntactic sugar is canonicalized, every virtual register is assigned exactly once, $\phi$-nodes merge values at branch join blocks, and every basic block ends with an explicit terminator (`Jump`, `Branch`, `Switch`, `Return`, or `Trap`).

##### **FR-008: Deterministic MIR Optimization Pipeline**

* **Description:** The system shall execute a configurable pipeline of optimization passes on the SSA MIR, including Constant Folding & Propagation, Algebraic Simplification & Strength Reduction, Common Subexpression Elimination (CSE), Memory-to-Register Promotion (`Mem2Reg`), Dead Code Elimination (DCE), and Leaf Function Inlining ($\le 25$ instructions).

* **Priority:** Should Have

* **Source Reference:** SRS Section 4.3 (`IR-OPT-001` through `IR-OPT-006`)

* **Dependencies:** FR-007

* **Acceptance Criteria:**

  * **Given** an MIR module containing constant arithmetic, multiplication by powers of two ($x \times 2^n$), non-escaping stack allocations, unreachable blocks, and a 15-instruction non-recursive leaf function,

  * **When** the MIR optimizer runs with optimizations enabled,

  * **Then** constants are folded using Wasm IEEE 754/two's complement semantics, $x \times 2^n$ is reduced to $x \ll n$, non-escaping slots are promoted to SSA registers, dead instructions/blocks are stripped, and the leaf function is inlined at call sites.

#### 4.3 Code Generation, Stackification & Binary Serialization

##### **FR-009: Structured Control Flow Recovery (Stackifier)**

* **Description:** The system shall ensure CFG reducibility and transform the MIR CFG into nested WebAssembly structured control blocks (`block`, `loop`, `if`) ordered in Reverse Post-Order (RPO), calculating relative label depths ($d \in \mathbb{N}_0$) for all `br` and `br_if` instructions.

* **Priority:** Must Have

* **Source Reference:** SRS Section 5.1 (`CG-SCF-001`, `CG-SCF-002`)

* **Dependencies:** FR-007, FR-008

* **Acceptance Criteria:**

  * **Given** an MIR CFG containing arbitrary forward merge branches and backward loop branches,

  * **When** the Stackifier pass executes,

  * **Then** any irreducible regions are split into reducible natural loops, backward branch targets are wrapped in `loop ... end` scopes, forward merge targets are wrapped in `block ... end` scopes, and all branches emit accurate relative depth indices.

##### **FR-010: Expression Stackification, `local.tee` Fusion & Register Coalescing**

* **Description:** The system shall keep single-use intra-block values directly on the Wasm operand stack (eliding `local.set`/`local.get`), fuse adjacent `local.set $x` + `local.get $x` pairs into `local.tee $x`, and coalesce multi-use SSA virtual registers into a minimal set of typed Wasm local variables (`i32`, `i64`, `f32`, `f64`).

* **Priority:** Must Have

* **Source Reference:** SRS Section 5.2 (`CG-STK-001`, `CG-STK-002`, `CG-STK-003`)

* **Dependencies:** FR-009

* **Acceptance Criteria:**

  * **Given** a structured LIR function containing single-use expression trees, an immediate local reuse, and non-overlapping cross-block SSA variables,

  * **When** local allocation and peephole optimization complete,

  * **Then** single-use temporaries bypass locals entirely, immediate reuses emit `local.tee`, and non-overlapping SSA lifetimes share the same Wasm local index.

##### **FR-011: Compliant Wasm Binary Encoding & Section Serialization**

* **Description:** The system shall emit a binary stream beginning with the 8-byte preamble (`\0asm` + version `1`), encode integers via `uLEB128`/`sLEB128` and floats via little-endian IEEE 754-2008, and serialize non-empty known sections in strict ascending numerical order (`1` through `9`, `12`, `10`, `11`, followed by Custom Section `0`).

* **Priority:** Must Have

* **Source Reference:** SRS Section 5.3 (`CG-BIN-001`, `CG-BIN-002`, `CG-BIN-003`)

* **Dependencies:** FR-010

* **Acceptance Criteria:**

  * **Given** a lowered LIR module ready for emission,

  * **When** the binary emitter serializes the `.wasm` file,

  * **Then** the file starts with `0x00 0x61 0x73 0x6D 0x01 0x00 0x00 0x00`, omits empty sections, orders active known sections strictly as `Type(1) -> Import(2) -> Function(3) -> Table(4) -> Memory(5) -> Global(6) -> Export(7) -> Start(8) -> Element(9) -> DataCount(12) -> Code(10) -> Data(11) -> Custom(0)`, and terminates every function body with `0x0B`.

#### 4.4 Memory Management & Linear Memory Layout

##### **FR-012: Four-Region Linear Memory Partitioning**

* **Description:** The system shall partition the 32-bit linear memory address space into four contiguous regions: an unmapped Null-Pointer Guard Zone (`0x00000000`–`0x000003FF`), a natural-aligned Static Data & BSS Region starting at `0x00000400`, a downward-growing Shadow Stack (default $1\text{ MiB}$) tracked by mutable global `$__stack_pointer`, and an upward-growing Dynamic Heap starting at `__heap_base`.

* **Priority:** Must Have

* **Source Reference:** SRS Section 6.1 (`MM-LAY-001`, `MM-LAY-002`, `MM-LAY-003`, `MM-LAY-004`)

* **Dependencies:** FR-011

* **Acceptance Criteria:**

  * **Given** a compiled module containing static string literals, shadow stack allocations, and dynamic heap usage,

  * **When** inspecting the emitted `Data` and `Global` sections,

  * **Then** no static data is placed below `0x00000400`, static segments are aligned to $1, 2, 4,$ or $8\text{ bytes}$, `$__stack_pointer` is initialized to `__heap_base`, and heap expansion invokes `memory.grow` in $64\text{ KiB}$ increments.

##### **FR-013: Shadow Stack Frame ABI & Calling Convention**

* **Description:** The system shall pass scalar primitives and pointers directly in Wasm registers, allocate aggregates (structs, arrays, unions) and escaped variables on the linear memory Shadow Stack, and emit 16-byte-aligned stack frame prologues and epilogues that adjust and restore `$__stack_pointer`.

* **Priority:** Must Have

* **Source Reference:** SRS Section 6.2 (`MM-ABI-001`, `MM-ABI-002`, `MM-ABI-003`)

* **Dependencies:** FR-006, FR-012

* **Acceptance Criteria:**

  * **Given** a function that declares an address-taken local struct requiring $K$ bytes of stack space,

  * **When** code generation emits the function body,

  * **Then** the function prologue decrements `$__stack_pointer` by a 16-byte-aligned $K$ and caches the frame pointer `$fp`, the body accesses struct fields via static offsets from `$fp`, and every return path restores `$__stack_pointer` to `$fp + K`.

##### **FR-014: Embedded Runtime Heap Allocator / GC Integration**

* **Description:** The system shall bundle a freestanding runtime memory manager directly into the `.wasm` module supporting either Manual/Ownership mode (TLSF or `dlmalloc` free-list allocator with $O(1)$ `malloc`/`free`) or Managed GC mode (non-moving mark-and-sweep collector scanning shadow stack roots).

* **Priority:** Should Have

* **Source Reference:** SRS Section 6.3 (`MM-ALC-001`)

* **Dependencies:** FR-012, FR-013

* **Acceptance Criteria:**

  * **Given** a source program performing dynamic heap allocations in Manual mode or Managed GC mode,

  * **When** compiled into a `.wasm` binary,

  * **Then** Manual mode embeds the $\sim 2\text{ KiB}$ TLSF/`dlmalloc` allocator backed by `memory.grow`, while Managed mode embeds the mark-and-sweep collector and registers live pointer-containing locals into shadow stack roots before GC-triggering calls.

#### 4.5 Runtime Integration, Host Interfacing & Diagnostics

##### **FR-015: Web Target JS/TS Bindings & Marshalling (`wasm32-unknown-web`)**

* **Description:** When targeting `wasm32-unknown-web`, the system shall generate an ES6 Module wrapper (`.js`) and TypeScript definitions (`.d.ts`) that automatically marshal UTF-8 strings and arrays across linear memory via `__alloc`/`__dealloc` and `TextEncoder`/`TextDecoder`, and manage opaque host/DOM objects via `externref` tables or a JS `HeapObjectTable`.

* **Priority:** Must Have

* **Source Reference:** SRS Section 7.1 & Section 7.2 (`RT-JS-001`, `RT-JS-002`, `RT-JS-003`)

* **Dependencies:** FR-011, FR-014

* **Acceptance Criteria:**

  * **Given** a source module exporting a function that accepts a string and a DOM object reference and returns a string, compiled with `--target wasm32-unknown-web`,

  * **When** the compiler builds the target artifacts,

  * **Then** it outputs `<module>.wasm`, `<module>.js`, and `<module>.d.ts`, where the JS wrapper automatically encodes input strings into Wasm memory via `__alloc`, resolves object handles/references, and decodes returned `(ptr, len)` slices via `TextDecoder` before invoking `__dealloc`.

##### **FR-016: WASI Target Entrypoint & Syscall Lowering (`wasm32-wasi`)**

* **Description:** When targeting `wasm32-wasi`, the system shall synthesize and export a `_start: () -> ()` entrypoint function (initializing the shadow stack, running static constructors, calling `main()`, and invoking `proc_exit`) and lower standard I/O, clock, and random operations to `wasi_snapshot_preview1` imports (`fd_write`, `fd_read`, `clock_time_get`, `random_get`).

* **Priority:** Must Have

* **Source Reference:** SRS Section 7.1 & Section 7.3 (`RT-WASI-001`, `RT-WASI-002`)

* **Dependencies:** FR-011, FR-013

* **Acceptance Criteria:**

  * **Given** a source program with a `main()` function that writes to standard output, compiled with `--target wasm32-wasi`,

  * **When** executed in a standalone WASI runtime (e.g., Wasmtime),

  * **Then** the runtime invokes the exported `_start` function, standard output is written via `wasi_snapshot_preview1.fd_write` using shadow-stack `ciovec` structs, and the process exits with the return code passed to `proc_exit`.

##### **FR-017: Saturating Casts, Debug Panic Hooks & Custom `"name"` Section**

* **Description:** The system shall emit non-trapping saturating float-to-int conversions (`0xFC` prefix) by default, and when `--debug` is enabled, inject explicit bounds/division/stack-overflow checks branching to `@__wasm_panic(msg_ptr, msg_len, file_ptr, line)` alongside a Custom `"name"` Section and DWARF/SourceMap metadata.

* **Priority:** Must Have

* **Source Reference:** SRS Section 7.4 (`RT-TRP-001`, `RT-TRP-002`, `RT-TRP-003`, `RT-TRP-004`)

* **Dependencies:** FR-011, FR-012

* **Acceptance Criteria:**

  * **Given** a program compiled with `--debug` that triggers a shadow stack overflow (`$__stack_pointer < $__data_end`) or out-of-bounds array access,

  * **When** executed in the target runtime,

  * **Then** the program branches to `@__wasm_panic` to log the exact source file, line number, and panic message to the host before trapping with `unreachable`, and the stack trace displays human-readable function names from the `"name"` Custom Section.

##### **FR-018: Throughput, Footprint & Deterministic Reproducibility**

* **Description:** The system shall compile source code at a minimum throughput of $50,000$ lines per second on a single modern CPU core, produce a dead-code-stripped "Hello World" binary under $512\text{ bytes}$ for `wasm32-unknown-web`, and guarantee bit-for-bit identical `.wasm` outputs across Linux, macOS, Windows, and self-hosted Wasm for identical inputs.

* **Priority:** Must Have

* **Source Reference:** SRS Section 8.1 (`NFR-PERF-001`, `NFR-PERF-002`, `NFR-PERF-003`) & Section 8.2

* **Dependencies:** FR-008, FR-011

* **Acceptance Criteria:**

  * **Given** an identical source codebase and compiler flags compiled on Linux, macOS, and Windows hosts,

  * **When** the resulting `.wasm` binaries are hashed (SHA-256) and validated via `wasm-validate`,

  * **Then** all three binaries pass validation, yield identical SHA-256 digests, and a minimal web "Hello World" artifact measures $< 512\text{ bytes}$.

### 5. Business Rules

| Rule ID | Rule Name | Description | Source Reference | 
| ----- | ----- | ----- | ----- | 
| **BR-001** | **Zero-LLVM Direct Emission** | The compiler must emit Wasm bytecode directly without invoking LLVM or Binaryen at any stage of compilation. | `SYS-ARCH-001` | 
| **BR-002** | **No Implicit Numeric Widening** | Operations between 32-bit and 64-bit integers or floats are prohibited unless an explicit `CastExpr` node is present in the typed HIR. | `FE-SEM-004` | 
| **BR-003** | **Escaped Variable Shadow Routing** | Any variable whose address is taken (`&x`) or that represents an aggregate type cannot reside in Wasm `local` registers and must be allocated on the linear memory shadow stack. | `FE-SEM-004`, `MM-ABI-002` | 
| **BR-004** | **Reducible Structured Control Flow** | Emitted Wasm code must never contain unstructured jumps; all control flow must resolve to reducible `block`, `loop`, and `if` scopes with relative branch depths. | `CG-SCF-001`, `CG-SCF-002` | 
| **BR-005** | **Strict Section Ordering & Omission** | Known sections must be serialized in exact order (`1..9, 12, 10, 11`, then Custom `0`), and any section with zero entries must be omitted entirely from the binary. | `CG-BIN-003` | 
| **BR-006** | **Null-Pointer Guard Protection** | Linear memory addresses `0x00000000` through `0x000003FF` ($1024\text{ bytes}$) are strictly reserved as an unmapped guard zone; static data must start at or above `0x00000400`. | `MM-LAY-001`, `MM-LAY-002` | 
| **BR-007** | **16-Byte Shadow Stack Alignment** | All shadow stack frame allocations ($K$ bytes) must be aligned to a 16-byte boundary and restored before every function return. | `MM-ABI-003` | 
| **BR-008** | **Default Non-Trapping Float Casts** | Float-to-integer conversions must default to non-trapping saturating opcodes (`i32.trunc_sat_f64_s`, prefix `0xFC`) unless explicitly overridden. | `RT-TRP-002` | 

### 6. Use Cases / User Flows

#### Use Case 1: End-to-End Compilation & Validation Pipeline

* **Actor:** Application Developer

* **Preconditions:** Developer has authored source file `main.src` and invokes the compiler CLI.

* **Postconditions:** Validated `.wasm` binary (and optional `.js`/`.d.ts` wrappers) written to disk.

* **Step-by-Step Flow:**

  1. **Lexing & Interning (`FR-001`, `FR-002`):** The compiler reads `main.src` into an immutable UTF-8 byte buffer, produces zero-copy span tokens, and interns identifiers into `StringInterner`. If lexical errors exist, they are logged and scanning continues.

  2. **Parsing & AST Construction (`FR-003`, `FR-004`):** The hybrid Recursive Descent / Pratt parser builds the flattened `Vec<AstNode>` array. Any syntax errors trigger panic-mode synchronization to `;` or `}` and insert `PoisonExpr` nodes.

  3. **Semantic Validation & Escape Analysis (`FR-005`, `FR-006`):** The bidirectional type checker validates types, suppresses secondary errors via `PoisonType`, verifies return paths, and marks `&x` variables as `EscapedToLinearMemory`. (If any front-end errors occurred in steps 1–3, compilation halts here and prints all formatted diagnostics).

  4. **MIR Lowering & Optimization (`FR-007`, `FR-008`):** The typed HIR is desugared and lowered into pruned SSA MIR. The optimizer runs `Mem2Reg`, Constant Folding, Strength Reduction, CSE, Inlining, and DCE.

  5. **Stackification & Local Allocation (`FR-009`, `FR-010`):** The Stackifier orders basic blocks in RPO, wraps loops and forward merges in `loop`/`block` scopes, elides locals for single-use expressions, fuses `local.tee` instructions, and coalesces remaining SSA registers via Linear Scan coloring.

  6. **Binary Serialization (`FR-011`, `FR-012`, `FR-013`):** The emitter writes the `\0asm` preamble, encodes sections in strict numerical order using LEB128, partitions linear memory (`0x00000400` static data, `$__stack_pointer`, `__heap_base`), and outputs the final `.wasm` binary.

#### Use Case 2: Browser Host String & Object Interop (`wasm32-unknown-web`)

* **Actors:** Host Integration Engineer (Browser JS Application), Target Wasm Runtime

* **Preconditions:** Module compiled with `--target wasm32-unknown-web`, producing `app.wasm`, `app.js`, and `app.d.ts`.

* **Step-by-Step Flow:**

  1. The Host JS application imports the generated ES6 function wrapper from `app.js` and calls `processUserData("Alice", domCanvasElement)`.

  2. **Host** $\rightarrow$ **Wasm String Marshalling (`FR-015`):** `app.js` encodes `"Alice"` into UTF-8 bytes via `TextEncoder`, calls Wasm's exported `__alloc(5, 1)`, copies the 5 bytes into `Uint8Array(wasmMemory.buffer, ptr, 5)`, and passes `(ptr, 5)` to the underlying Wasm export.

  3. **Host Object Reference Passing (`FR-015`):** `app.js` inserts `domCanvasElement` into the `externref` table (or `HeapObjectTable` slab) and passes the 32-bit handle index to Wasm.

  4. **Wasm Execution & Return (`FR-015`):** The Wasm function executes, allocates a response string in linear memory, and returns a packed `i64` containing `(ret_ptr, ret_len)`.

  5. **Wasm** $\rightarrow$ **Host Cleanup (`FR-015`):** `app.js` decodes the response string via `TextDecoder.decode(new Uint8Array(wasmMemory.buffer, ret_ptr, ret_len))`, invokes `__dealloc(ret_ptr, ret_len)` to prevent memory leaks, and returns the native JavaScript string to the caller.

#### Use Case 3: Debug Panic Diagnostics on Shadow Stack Overflow

* **Actors:** Application Developer, Target Wasm Runtime

* **Preconditions:** Module compiled with `--debug` flag enabled.

* **Step-by-Step Flow:**

  1. A deeply recursive function executes its prologue, subtracting frame size $K$ from `$__stack_pointer` (`FR-013`).

  2. The compiler-injected debug check detects that `$__stack_pointer < $__data_end` (`FR-017`).

  3. Instead of silently corrupting static data in `.ro`/`.bss`, execution branches to the `@__wasm_panic(msg_ptr, msg_len, file_ptr, line)` intrinsic.

  4. `@__wasm_panic` invokes the imported host logger with the static message `"Shadow stack overflow"` along with the offending source filename and line number, then executes the `unreachable` (`0x00`) opcode.

  5. The runtime halts execution and prints a stack trace featuring exact source function names resolved from the Custom `"name"` Section (`FR-017`).

### 7. Data Requirements & Core Entities

| Entity Name | Key Attributes | Relationships & Lifecycle | 
| ----- | ----- | ----- | 
| **Token & StringInterner** | `token_kind`, `start_offset: u32`, `length: u32`, `SymbolId: u32` | Zero-copy slices into the immutable UTF-8 source buffer; deduplicated identifiers stored in an arena-backed hash table (`FE-LEX-002`, `FE-LEX-003`). | 
| **Flattened AST Pool** | `Vec<AstNode>` (`NodeIndex: u32`), parallel `Vec<Span>`, parallel `Vec<TypeId>` | Struct-of-Arrays topology where child nodes are referenced by 32-bit index rather than heap pointers (`FE-AST-001`, `FE-AST-002`). | 
| **Scoped Symbol Table** | `scope_id`, `parent_scope_index`, `Map<SymbolId, SymbolEntry>`, `is_escaped: bool` | Cactus stack of lexical scopes tracking variable types and flagging address-taken variables (`EscapedToLinearMemory`) (`FE-SEM-001`, `FE-SEM-004`). | 
| **SSA MIR Graph** | `BlockId`, `VirtualReg (%vN: Type)`, `PhiNodes[]`, `Instructions[]`, `Terminator` | Control Flow Graph in pruned Static Single Assignment form; transformed by optimization passes and consumed by the Stackifier (`IR-MIR-001`..`003`). | 
| **Linear Memory Image** | `GuardZone (0x0..0x3FF)`, `StaticData (0x400..__data_end)`, `ShadowStack (__data_end..__heap_base)`, `Heap (__heap_base..)` | 32-bit byte-addressable memory space managed at runtime via `$__stack_pointer` and `memory.grow` (`MM-LAY-001`..`004`). | 
| **Wasm Binary Artifact** | `Magic (\0asm)`, `Version (1)`, `Sections[1..9, 12, 10, 11]`, `CustomSection(0)` | Final serialized byte stream formatted with LEB128 and IEEE 754 encodings (`CG-BIN-001`..`003`). | 

### 8. Assumptions, Constraints & Open Issues

#### 8.1 Assumptions

* **A-001:** Source code input files are provided as contiguous, valid UTF-8 byte buffers (or invalid sequences are meant to be flagged as lexical errors per `FE-LEX-005`).

* **A-002:** Target execution environments support at minimum the WebAssembly 1.0/2.0 Core Specification (including non-trapping float-to-int conversions `0xFC` and optional `externref` reference types).

#### 8.2 Constraints

* **C-001 (Zero Third-Party Backend):** The compiler is strictly prohibited from linking against LLVM or Binaryen (`SYS-ARCH-001`).

* **C-002 (Pointer-Free Internal Arenas):** Compiler internal data structures (AST, IR, Symbol Tables) must use arena allocation or flattened index-based arrays (`uint32_t`) rather than pointer graphs (`SYS-ARCH-002`).

* **C-003 (32-Bit Address Space):** The target architecture is `wasm32`, utilizing 32-bit pointers and $64\text{ KiB}$ memory page granularity.

#### 8.3 Open Issues Requiring Stakeholder Confirmation

* **OI-001 (Source Language Concrete Grammar):** Confirm the formal EBNF grammar of the source language (keywords, operator symbols, and whether generics require trait/interface bounds during monomorphization in Section 4.1).

* **OI-002 (Allocator Mode Selection Flag):** Confirm whether selecting between **Mode A** (TLSF Manual Allocator) and **Mode B** (Mark-and-Sweep GC) in `MM-ALC-001` is controlled via a compiler CLI flag (e.g., `--memory-model=manual|gc`) or language-level annotations.

* **OI-003 (WAT Text Emission Flag):** Section 1.2 states the compiler outputs `.wasm` and optional WebAssembly Text format (`.wat`). Confirm the CLI flag (e.g., `--emit-wat`) and whether `.wat` emission is required for the MVP release.

### 9. Requirement Traceability Matrix (RTM)

| FRD ID | Requirement Title | Originating SRS ID(s) | SRS Section | Priority | Verification Method | 
| ----- | ----- | ----- | ----- | ----- | ----- | 
| **FR-001** | Zero-Copy Source Tokenization & Interning | `FE-LEX-001`, `FE-LEX-002`, `FE-LEX-003`, `FE-LEX-004` | Sec 3.1 | Must | Unit Test & Heap Allocation Profiler | 
| **FR-002** | Non-Halting Lexical Error Recovery | `FE-LEX-005` | Sec 3.1 | Must | Negative Fuzz & Multi-Error Test | 
| **FR-003** | Syntactic Parsing & Operator Precedence Resolution | `FE-PAR-001`, `FE-PAR-002`, `FE-AST-001`, `FE-AST-002` | Sec 3.2, 3.3 | Must | AST Index Snapshot Test | 
| **FR-004** | Panic-Mode Syntax Error Synchronization | `FE-PAR-003` | Sec 3.2 | Must | Negative Syntax Recovery Suite | 
| **FR-005** | Scoped Symbol Resolution & Bidirectional Type Checking | `FE-SEM-001`, `FE-SEM-002`, `FE-SEM-003` | Sec 3.4 | Must | Semantic Unification & Poison Test | 
| **FR-006** | Wasm Semantic Pre-Validation & Escape Analysis | `FE-SEM-004` | Sec 3.4 | Must | Escape Analysis & Cast Verification | 
| **FR-007** | HIR Desugaring & SSA MIR Construction | `IR-MIR-001`, `IR-MIR-002`, `IR-MIR-003` | Sec 4.1, 4.2 | Must | CFG Dominance & SSA Invariant Check | 
| **FR-008** | Deterministic MIR Optimization Pipeline | `IR-OPT-001` through `IR-OPT-006` | Sec 4.3 | Should | Differential Execution (`-O0` vs `-O3`) | 
| **FR-009** | Structured Control Flow Recovery (Stackifier) | `CG-SCF-001`, `CG-SCF-002` | Sec 5.1 | Must | `wasm-validate` & CFG Fuzzing | 
| **FR-010** | Expression Stackification, `local.tee` Fusion & Coalescing | `CG-STK-001`, `CG-STK-002`, `CG-STK-003` | Sec 5.2 | Must | Bytecode Inspection & Local Count Test | 
| **FR-011** | Compliant Wasm Binary Encoding & Section Serialization | `CG-BIN-001`, `CG-BIN-002`, `CG-BIN-003` | Sec 5.3 | Must | WABT `wasm-validate` & `wasm-tools` | 
| **FR-012** | Four-Region Linear Memory Partitioning | `MM-LAY-001`, `MM-LAY-002`, `MM-LAY-003`, `MM-LAY-004` | Sec 6.1 | Must | Binary Section & Memory Map Inspection | 
| **FR-013** | Shadow Stack Frame ABI & Calling Convention | `MM-ABI-001`, `MM-ABI-002`, `MM-ABI-003` | Sec 6.2 | Must | Aggregate Passing & Alignment Test | 
| **FR-014** | Embedded Runtime Heap Allocator / GC Integration | `MM-ALC-001` | Sec 6.3 | Should | Heap Stress & GC Root Retention Test | 
| **FR-015** | Web Target JS/TS Bindings & Marshalling | `RT-JS-001`, `RT-JS-002`, `RT-JS-003` | Sec 7.1, 7.2 | Must | Node.js / V8 FFI Integration Test | 
| **FR-016** | WASI Target Entrypoint & Syscall Lowering | `RT-WASI-001`, `RT-WASI-002` | Sec 7.1, 7.3 | Must | Wasmtime CLI Execution Test | 
| **FR-017** | Saturating Casts, Debug Panic Hooks & `"name"` Section | `RT-TRP-001`, `RT-TRP-002`, `RT-TRP-003`, `RT-TRP-004` | Sec 7.4 | Must | Trap Injection & Stack Trace Test | 
| **FR-018** | Throughput, Footprint & Deterministic Reproducibility | `SYS-ARCH-001`, `SYS-ARCH-002`, `NFR-PERF-001`..`003` | Sec 2.2, 8.1, 8.2 | Must | Cross-OS CI Benchmark & SHA-256 Check | 
