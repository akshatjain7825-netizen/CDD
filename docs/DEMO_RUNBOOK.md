# Interactive Live Demonstration Runbook — `wasmc`

> **Living Docs Role: Operational Runbook**
> Governed under ECC `living-docs-governance`. This document provides the authoritative, step-by-step operational procedure to reproduce and demonstrate all compiler capabilities, diagnostics, CLI subcommands, caching mechanisms, daemon REST APIs, and runtime execution.

---

## 1. Environment & Prerequisites

### 1.1 Toolchain Requirements
- **Rust Toolchain:** Rust 1.85+ (Edition 2024, `stable-x86_64-pc-windows-gnu` or `stable-x86_64-unknown-linux-gnu`)
- **C/LLD Linker:** LLVM-MinGW UCRT x86_64 Clang/LLD
- **Runtime Engine:** Node.js v22+ (v24.19.0 recommended, with native WebAssembly Core 2.0 V8 engine)
- **Shell:** PowerShell 7+ (`pwsh`) or Bash

### 1.2 Path Configuration (PowerShell)
Execute within the `CDD` workspace root (`d:\projects\CD\CDD`):
```powershell
$mingwBin = "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\MartinStorsjo.LLVM-MinGW.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\llvm-mingw-20260908-ucrt-x86_64\bin"
$cargoBin = "$env:USERPROFILE\.cargo\bin"
$env:PATH = "$cargoBin;$mingwBin;$env:PATH"
$env:WASMC_BIN = "$PWD\target\debug\wasmc.exe"
```

---

## 2. Step-by-Step Demonstration Sequence

### Step 1: Workspace Compilation & Unit Testing across all 16 Crates
Compile the full toolchain and verify all 30 unit, integration, and doc tests:
```powershell
# 1. Compile debug binary
cargo build --bin wasmc

# 2. Run all workspace unit and integration tests
cargo test --workspace
```
*Expected Output:*
- `30 passed; 0 failed; 0 warnings across all 16 crates`.

---

### Step 2: Front-End Validation (`wasmc check`) & Intentional Error Diagnostics
Validate valid code, then demonstrate the compiler's rich ANSI diagnostic sink and recovery mechanisms:

#### A. Successful Static Check
```powershell
& $env:WASMC_BIN check examples/control_flow.src
```
*Expected Output:* Clean exit with exit code `0`.

#### B. Lexical Diagnostic (`E1001` - Invalid Character)
```powershell
"export fn test() -> void { let x = @; }" | Out-File -Encoding utf8 scratch_err.src
& $env:WASMC_BIN check scratch_err.src
```
*Expected Output:* Diagnostic `error[E1001]: Unrecognized token '@'`.

#### C. Syntactic Diagnostic (`E2001` - Syntax Parse Error)
```powershell
"export fn test() -> void { let x = 42 }" | Out-File -Encoding utf8 scratch_err.src
& $env:WASMC_BIN check scratch_err.src
```
*Expected Output:* Diagnostic `error[E2001]: Expected ';' after statement`.

#### D. Semantic Diagnostic (`E3002` - Type Mismatch)
```powershell
"export fn test() -> i32 { return 3.14; }" | Out-File -Encoding utf8 scratch_err.src
& $env:WASMC_BIN check scratch_err.src
```
*Expected Output:* Diagnostic `error[E3002]: Type mismatch: expected i32, found f64`.

#### E. Machine-Readable JSON Diagnostics (`--error-format json`)
```powershell
& $env:WASMC_BIN check scratch_err.src --error-format json
Remove-Item scratch_err.src -ErrorAction SilentlyContinue
```
*Expected Output:* Structured JSON array with `code`, `severity`, `span`, and `message` fields.

---

### Step 3: Direct-to-Wasm Binary Compilation across all 6 Examples
Compile all reference source programs directly to binary WebAssembly (`.wasm`) and WebAssembly Text format (`.wat`):

```powershell
# 1. Hello World (Host print import & string data segment)
& $env:WASMC_BIN build examples/hello.src -o examples/hello.wasm --emit-wat

# 2. Arithmetic & Floating Point Operations
& $env:WASMC_BIN build examples/arithmetic.src -o examples/arithmetic.wasm --emit-wat

# 3. Control Flow & Loops (while, if, branch inversion)
& $env:WASMC_BIN build examples/control_flow.src -o examples/control_flow.wasm --emit-wat

# 4. Functions & Tail Calls
& $env:WASMC_BIN build examples/functions.src -o examples/functions.wasm --emit-wat

# 5. Memory & Shadow Stack (Address-of &, pointer deref *)
& $env:WASMC_BIN build examples/memory.src -o examples/memory.wasm --emit-wat

# 6. Sorting Algorithms (Bubble sort on shadow-stack array)
& $env:WASMC_BIN build examples/sorting.src -o examples/sorting.wasm --emit-wat
```
*Expected Verification:*
Every output `.wasm` file starts with the standard 4-byte magic number `\0asm` (`0x00 0x61 0x73 0x6d`) followed by WebAssembly version `1` (`0x01 0x00 0x00 0x00`).

---

### Step 4: Incremental Compilation Caching (`wasmc-cache`)
Verify that the BLAKE3-keyed SQLite 3 WAL cache avoids redundant recompilations:

```powershell
# Cold compilation into fresh cache directory
& $env:WASMC_BIN build examples/sorting.src -o examples/sorting.wasm --cache-dir .wasmc-cache

# Warm compilation (Instant cache hit)
& $env:WASMC_BIN build examples/sorting.src -o examples/sorting.wasm --cache-dir .wasmc-cache
```
*Expected Verification:*
The SQLite WAL database `.wasmc-cache/cache.db` tracks compilation units, flag hashes, and artifact blobs. Stored blobs match exactly on identical compiler flags and source hashes.

---

### Step 5: Standalone Node.js V8 WebAssembly Runtime Execution
Execute compiled WebAssembly binaries directly on Node.js's V8 engine using [`scripts/run_wasm.js`](../scripts/run_wasm.js):

#### A. Execute `sorting.wasm`
```powershell
node scripts/run_wasm.js examples/sorting.wasm
```
*Expected Output:*
- Initial Array: `[9, 3, 7, 1, 8, 2, 5]`
- Sorted Array:  `[1, 2, 3, 5, 7, 8, 9]`
- `collatz_steps(27) = 111`
- `gcd(48, 18) = 6`

#### B. Execute `hello.wasm`
```powershell
node scripts/run_wasm.js examples/hello.wasm
```
*Expected Output:*
- `[Host Print]: "Hello, WebAssembly!"`
- `main() exit code = 0`

---

### Step 6: Axum HTTP/2 Compiler REST Daemon (`wasmc daemon`)
Start the embedded Axum HTTP REST server and verify all endpoints:

#### A. Launch the Daemon in the Background
```powershell
$daemonProc = Start-Process -FilePath $env:WASMC_BIN -ArgumentList "daemon --host 127.0.0.1 --port 8080 --cache-dir .wasmc-cache" -PassThru
Start-Sleep -Seconds 1
```

#### B. Health Probe (`GET /api/v1/health`)
```powershell
Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/health" -Method Get
```
*Expected Output:* `@{status=UP}`

#### C. Remote Typecheck (`POST /api/v1/check`)
```powershell
$checkBody = @{
    sources = @(
        @{ path = "remote.src"; content_utf8 = "export fn compute(x: i32) -> i32 { return x * 2; }" }
    )
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/check" -Method Post -Body $checkBody -ContentType "application/json"
```
*Expected Output:* `@{status=SUCCESS; diagnostics=@()}`

#### D. Remote Compilation (`POST /api/v1/compile`)
```powershell
$compileBody = @{
    project_id = "demo_proj"
    target = "wasm32-unknown-web"
    opt_level = "O2"
    sources = @(
        @{ path = "math.src"; content_utf8 = "export fn mult(a: i32, b: i32) -> i32 { return a * b; }" }
    )
} | ConvertTo-Json

$resp = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/compile" -Method Post -Body $compileBody -ContentType "application/json"
Write-Host "Compile Status:" $resp.status
Write-Host "BLAKE3 Hash:   " $resp.artifacts.wasm_blake3_hash
Write-Host "Binary Size:   " $resp.artifacts.wasm_size_bytes "bytes"
```
*Expected Output:* `status: SUCCESS`, valid BLAKE3 hash, non-zero byte size.

#### E. Cache Status Inspection (`GET /api/v1/cache/status`)
```powershell
Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/cache/status" -Method Get
```
*Expected Output:* JSON object showing `total_units`, `total_functions`, `total_sessions`, and `db_size_bytes`.

#### F. Terminate Daemon Process
```powershell
Stop-Process -Id $daemonProc.Id -Force
```

---

### Step 7: Automated 39-Test End-to-End Suite Validation
Run the comprehensive regression and end-to-end dry run test suite:
```powershell
node scripts/complete_dry_run_test.js
```
*Expected Output:*
```
================================================================
COMPLETE DRY RUN TEST SUITE FINISHED: 39/39 TESTS PASSED
================================================================
```

---

### Step 8: Regenerate & Launch Interactive Browser Studio (`demo.html`)
Build the self-contained zero-dependency browser studio HTML artifact:
```powershell
# 1. Regenerate bundle from latest compiled examples
node scripts/build_demo_html.js

# 2. Launch in default web browser
Start-Process "demo.html"
```
*Features in Studio:*
- Multi-example switcher (`Sorting`, `Functions`, `Control Flow`, `Memory`, `Arithmetic`, `Hello`)
- Live split-view: High-level `.src` source code side-by-side with emitted `.wat` structured assembly
- Live browser-side WebAssembly execution with real-time DOM console
- Memory layout visualizer showing Null Guard, Data Segment, Shadow Stack, and Heap Base.
