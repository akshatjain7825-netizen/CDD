# Automated Test Report & Verification Evidence — `wasmc`

## 1. Test Execution Summary

- **Date of Run:** 2026-10-15
- **Toolchain:** Rust 1.99.0 (`stable-x86_64-pc-windows-gnu`)
- **Linker:** LLVM-MinGW UCRT 64-bit Clang/LLD
- **Host Architecture:** Windows 11 x86_64
- **Runtime Validator / Engine:** Node.js v24.19.0 (V8 WebAssembly Core 2.0 Engine)
- **Overall Suite Status:** **37 / 37 TESTS PASSED (100% Pass Rate)**

---

## 2. Workspace Integration Test Suite Results

Command:
```powershell
cargo test --package wasmc-driver --test integration_tests
```

```
running 21 tests
test test_end_to_end_floating_point ... ok
test test_early_returns_inside_loops ... ok
test test_end_to_end_arithmetic_compilation ... ok
test test_end_to_end_control_flow_and_loops ... ok
test test_browser_bindings_and_typescript_generation ... ok
test test_deterministic_byte_reproducibility ... ok
test test_leb128_codec_boundaries ... ok
test test_lexer_tokenization_and_recovery ... ok
test test_parser_operator_precedence ... ok
test test_minimal_valid_module ... ok
test test_recursive_fibonacci ... ok
test test_semantics_duplicate_symbol_error ... ok
test test_nested_function_calls_multiple_params ... ok
test test_nested_loops_and_conditionals ... ok
test test_semantics_type_mismatch_error ... ok
test test_semantics_unresolved_symbol_error ... ok
test test_shadow_stack_and_pointer_dereference ... ok
test test_short_circuit_boolean_expressions ... ok
test test_wasi_runner_shim_generation ... ok
test test_invalid_syntax_error_reporting ... ok
test test_incremental_cache_sqlite ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

---

## 3. Complete End-to-End Dry Run Results (`complete_dry_run_test.js`)

Command:
```powershell
node scripts/complete_dry_run_test.js
```

### Execution Output:
```
================================================================
       STARTING COMPLETE DRY RUN TEST FOR WASMC TOOLCHAIN       
================================================================

--- Step 1: CLI Version and Subcommand Checks ---
  PASS: wasmc --help displays correct header
  PASS: wasmc subcommands present

--- Step 2: Direct-to-Wasm Binary Compilation ---
  PASS: Successfully emitted examples/arithmetic.wasm
  PASS: examples/arithmetic.wasm has valid \0asm magic bytes
  PASS: examples/arithmetic.wasm has valid Wasm Version 1 header
  PASS: Successfully emitted examples/control_flow.wasm
  PASS: examples/control_flow.wasm has valid \0asm magic bytes
  PASS: examples/control_flow.wasm has valid Wasm Version 1 header
  PASS: Successfully emitted examples/functions.wasm
  PASS: examples/functions.wasm has valid \0asm magic bytes
  PASS: examples/functions.wasm has valid Wasm Version 1 header
  PASS: Successfully emitted examples/hello.wasm
  PASS: examples/hello.wasm has valid \0asm magic bytes
  PASS: examples/hello.wasm has valid Wasm Version 1 header
  PASS: Successfully emitted examples/memory.wasm
  PASS: examples/memory.wasm has valid \0asm magic bytes
  PASS: examples/memory.wasm has valid Wasm Version 1 header

--- Step 3: Node.js V8 WebAssembly Runtime Execution & Assertions ---
  PASS: arithmetic.wasm: add(40, 2) == 42
  PASS: arithmetic.wasm: compute(10, 20) == 205
  PASS: arithmetic.wasm: float_calc(2.0, 3.0) == 7.0
  PASS: control_flow.wasm: sum_up_to(10) == 55 (1+2+...+10)
  PASS: control_flow.wasm: count_even(10) == 5 (0,2,4,6,8)
  PASS: functions.wasm: sum_of_squares(3, 4) == 25 (3^2 + 4^2)
  PASS: functions.wasm: factorial(5) == 120 (5!)
  PASS: memory.wasm: stack_math(10, 20) == 72 (address-of local)
  PASS: hello.wasm: main() returns 0
  PASS: hello.wasm: host print callback was invoked
  PASS: hello.wasm: correct string ptr (1408) and length (19) passed

--- Step 4: wasmc check Valid and Invalid Verification ---
  PASS: wasmc check on valid source succeeds
  PASS: wasmc check on non-existent or invalid source returns non-zero code

--- Step 5: Compiler Daemon HTTP/2 REST API Verification ---
  PASS: GET /api/v1/health returns HTTP 200 UP
  PASS: POST /api/v1/compile returns HTTP 200
  PASS: POST /api/v1/compile status is SUCCESS
  PASS: POST /api/v1/compile returned base64 wasm
  PASS: POST /api/v1/compile returned BLAKE3 hash
  PASS: Daemon compiled module: remote_mult(6, 7) == 42
  PASS: GET /api/v1/cache/status returns HTTP 200

================================================================
COMPLETE DRY RUN TEST SUITE FINISHED: 37/37 TESTS PASSED
================================================================
```

---

## 4. Deterministic Reproducibility

Two independent compilations of the identical source without cache produce identical byte streams:
- Compilation 1 BLAKE3: `a37fdaf023ae3191`
- Compilation 2 BLAKE3: `a37fdaf023ae3191`
- Difference: 0 bytes.
**Status: PASS**
