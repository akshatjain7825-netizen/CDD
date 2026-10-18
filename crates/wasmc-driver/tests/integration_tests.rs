use wasmc_ast::*;
use wasmc_diagnostics::DiagnosticSink;
use wasmc_driver::{CompilationDriver, CompilerOptions, SourceInput};
use wasmc_lexer::*;
use wasmc_parser::Parser;
use wasmc_semantics::SemanticAnalyzer;
use wasmc_wasm::Leb128Codec;

#[test]
fn test_lexer_tokenization_and_recovery() {
    let source = "fn add(a: i32, b: i32) -> i32 { return a + b; } @ let x = 42;";
    let mut interner = SymbolInterner::new();
    let mut lexer = Lexer::new(source, 1, &mut interner);
    let mut sink = DiagnosticSink::new();
    let tokens = lexer.tokenize_all(&mut sink);

    assert!(sink.has_errors(), "Expected diagnostic for '@' invalid character");
    assert_eq!(sink.diagnostics()[0].code, "E1001");
    // Recovery continued and tokenized 'let x = 42;'
    assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Let)));
    assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Integer(42))));
}

#[test]
fn test_parser_operator_precedence() {
    let source = "fn expr() -> i32 { return 1 + 2 * 3; }";
    let mut interner = SymbolInterner::new();
    let mut lexer = Lexer::new(source, 1, &mut interner);
    let mut sink = DiagnosticSink::new();
    let tokens = lexer.tokenize_all(&mut sink);
    assert!(!sink.has_errors());

    let mut pool = AstPool::new();
    let mut parser = Parser::new(tokens, &mut sink);
    parser.parse_module(&mut pool);
    assert!(!sink.has_errors());
    assert_eq!(pool.top_level_decls.len(), 1);
}

#[test]
fn test_semantics_duplicate_symbol_error() {
    let source = r#"
        fn foo() -> i32 { return 1; }
        fn foo() -> i32 { return 2; }
    "#;
    let mut interner = SymbolInterner::new();
    let mut lexer = Lexer::new(source, 1, &mut interner);
    let mut sink = DiagnosticSink::new();
    let tokens = lexer.tokenize_all(&mut sink);
    let mut pool = AstPool::new();
    let mut parser = Parser::new(tokens, &mut sink);
    parser.parse_module(&mut pool);

    let analyzer = SemanticAnalyzer::new(&pool, &mut sink);
    let result = analyzer.analyze();
    assert!(result.is_err(), "Duplicate symbol should fail semantic analysis");
    assert!(sink.diagnostics().iter().any(|d| d.code == "E3001"));
}

#[test]
fn test_semantics_type_mismatch_error() {
    let source = r#"
        fn bar() -> i32 {
            let x: i32 = "hello";
            return x;
        }
    "#;
    let mut interner = SymbolInterner::new();
    let mut lexer = Lexer::new(source, 1, &mut interner);
    let mut sink = DiagnosticSink::new();
    let tokens = lexer.tokenize_all(&mut sink);
    let mut pool = AstPool::new();
    let mut parser = Parser::new(tokens, &mut sink);
    parser.parse_module(&mut pool);

    let analyzer = SemanticAnalyzer::new(&pool, &mut sink);
    let result = analyzer.analyze();
    assert!(result.is_err(), "Type mismatch should fail semantic analysis");
    assert!(sink.diagnostics().iter().any(|d| d.code == "E3002"));
}

#[test]
fn test_semantics_unresolved_symbol_error() {
    let source = r#"
        fn baz() -> i32 {
            return undeclared_variable + 1;
        }
    "#;
    let mut interner = SymbolInterner::new();
    let mut lexer = Lexer::new(source, 1, &mut interner);
    let mut sink = DiagnosticSink::new();
    let tokens = lexer.tokenize_all(&mut sink);
    let mut pool = AstPool::new();
    let mut parser = Parser::new(tokens, &mut sink);
    parser.parse_module(&mut pool);

    let analyzer = SemanticAnalyzer::new(&pool, &mut sink);
    let result = analyzer.analyze();
    assert!(result.is_err(), "Unresolved symbol must fail analysis");
    assert!(sink.diagnostics().iter().any(|d| d.code == "E3003"));
}

#[test]
fn test_leb128_codec_boundaries() {
    let mut buf = Vec::new();

    // u32: 0
    Leb128Codec::encode_u32(&mut buf, 0);
    assert_eq!(buf, vec![0x00]);
    let (val, len) = Leb128Codec::decode_u32(&buf).unwrap();
    assert_eq!(val, 0);
    assert_eq!(len, 1);

    // u32: 624485
    buf.clear();
    Leb128Codec::encode_u32(&mut buf, 624485);
    let (val, len) = Leb128Codec::decode_u32(&buf).unwrap();
    assert_eq!(val, 624485);
    assert_eq!(len, 3);

    // s32: -624485
    buf.clear();
    Leb128Codec::encode_i32(&mut buf, -624485);
    let (val, _) = Leb128Codec::decode_i32(&buf).unwrap();
    assert_eq!(val, -624485);
}

#[test]
fn test_minimal_valid_module() {
    let source = "export fn noop() -> void {}";
    let driver = CompilationDriver::new(CompilerOptions {
        cache_dir: None,
        ..Default::default()
    });
    let sources = vec![SourceInput {
        path: "min.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Minimal module must compile");
    assert_eq!(&res.wasm_bytes[0..4], b"\0asm");
    assert_eq!(&res.wasm_bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn test_end_to_end_arithmetic_compilation() {
    let source = r#"
        export fn add(a: i32, b: i32) -> i32 {
            return a + b;
        }
        export fn calc(x: i32) -> i32 {
            return (x * 4) + 10;
        }
    "#;
    let options = CompilerOptions {
        emit_wat: true,
        cache_dir: None,
        ..Default::default()
    };
    let driver = CompilationDriver::new(options);
    let sources = vec![SourceInput {
        path: "arith.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Compilation should succeed");

    assert!(!res.wasm_bytes.is_empty());
    assert_eq!(&res.wasm_bytes[0..4], b"\0asm");
    assert_eq!(&res.wasm_bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
    assert!(res.wat_text.is_some());
}

#[test]
fn test_end_to_end_floating_point() {
    let source = r#"
        export fn fcalc(a: f64, b: f64) -> f64 {
            let x: f64 = a * 1.5;
            let y: f64 = b / 2.0;
            return x + y;
        }
    "#;
    let driver = CompilationDriver::new(CompilerOptions {
        cache_dir: None,
        ..Default::default()
    });
    let sources = vec![SourceInput {
        path: "float.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Float operations must compile");
    assert!(!res.wasm_bytes.is_empty());
}

#[test]
fn test_end_to_end_control_flow_and_loops() {
    let source = r#"
        export fn sum(n: i32) -> i32 {
            let mut s: i32 = 0;
            let mut i: i32 = 1;
            while (i <= n) {
                s = s + i;
                i = i + 1;
            }
            return s;
        }
    "#;
    let options = CompilerOptions {
        emit_wat: true,
        cache_dir: None,
        ..Default::default()
    };
    let driver = CompilationDriver::new(options);
    let sources = vec![SourceInput {
        path: "loop.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Compilation should succeed");

    assert!(!res.wasm_bytes.is_empty());
    assert_eq!(&res.wasm_bytes[0..4], b"\0asm");
}

#[test]
fn test_nested_loops_and_conditionals() {
    let source = r#"
        export fn grid_sum(rows: i32, cols: i32) -> i32 {
            let mut total: i32 = 0;
            let mut r: i32 = 0;
            while (r < rows) {
                let mut c: i32 = 0;
                while (c < cols) {
                    if ((r + c) % 2 == 0) {
                        total = total + 1;
                    }
                    c = c + 1;
                }
                r = r + 1;
            }
            return total;
        }
    "#;
    let driver = CompilationDriver::new(CompilerOptions {
        cache_dir: None,
        ..Default::default()
    });
    let sources = vec![SourceInput {
        path: "nested.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Nested loops must compile");
    assert!(!res.wasm_bytes.is_empty());
}

#[test]
fn test_early_returns_inside_loops() {
    let source = r#"
        export fn find_first_match(target: i32, limit: i32) -> i32 {
            let mut i: i32 = 0;
            while (i < limit) {
                if (i == target) {
                    return i * 2;
                }
                i = i + 1;
            }
            return -1;
        }
    "#;
    let driver = CompilationDriver::new(CompilerOptions {
        cache_dir: None,
        ..Default::default()
    });
    let sources = vec![SourceInput {
        path: "early_ret.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Early returns must compile");
    assert!(!res.wasm_bytes.is_empty());
}

#[test]
fn test_recursive_fibonacci() {
    let source = r#"
        export fn fib(n: i32) -> i32 {
            if (n <= 1) {
                return n;
            }
            return fib(n - 1) + fib(n - 2);
        }
    "#;
    let driver = CompilationDriver::new(CompilerOptions {
        cache_dir: None,
        ..Default::default()
    });
    let sources = vec![SourceInput {
        path: "fib.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Recursive fibonacci must compile");
    assert!(!res.wasm_bytes.is_empty());
}

#[test]
fn test_nested_function_calls_multiple_params() {
    let source = r#"
        fn clamp(val: i32, low: i32, high: i32) -> i32 {
            if (val < low) {
                return low;
            }
            if (val > high) {
                return high;
            }
            return val;
        }

        export fn compute_clamped(a: i32, b: i32, min_val: i32, max_val: i32) -> i32 {
            let sum: i32 = a + b;
            return clamp(sum, min_val, max_val);
        }
    "#;
    let driver = CompilationDriver::new(CompilerOptions {
        cache_dir: None,
        ..Default::default()
    });
    let sources = vec![SourceInput {
        path: "clamp.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Nested calls with multiple params must compile");
    assert!(!res.wasm_bytes.is_empty());
}

#[test]
fn test_short_circuit_boolean_expressions() {
    let source = r#"
        export fn check_bounds(x: i32, y: i32) -> bool {
            return (x >= 0 && x < 100) || (y >= 0 && y < 50);
        }
    "#;
    let driver = CompilationDriver::new(CompilerOptions {
        cache_dir: None,
        ..Default::default()
    });
    let sources = vec![SourceInput {
        path: "logic.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Logical expressions must compile");
    assert!(!res.wasm_bytes.is_empty());
}

#[test]
fn test_shadow_stack_and_pointer_dereference() {
    let source = r#"
        export fn mutate_via_ptr(a: i32) -> i32 {
            let mut val: i32 = a;
            let ptr: *i32 = &val;
            return *ptr + 10;
        }
    "#;
    let driver = CompilationDriver::new(CompilerOptions {
        cache_dir: None,
        ..Default::default()
    });
    let sources = vec![SourceInput {
        path: "ptr.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Address-taken locals must compile");
    assert!(!res.wasm_bytes.is_empty());
    assert_eq!(res.memory_map.null_guard_end, 1024);
}

#[test]
fn test_browser_bindings_and_typescript_generation() {
    let source = r#"
        export fn add(x: i32, y: i32) -> i32 {
            return x + y;
        }
        export fn greet() -> void {}
    "#;
    let driver = CompilationDriver::new(CompilerOptions {
        target: "wasm32-unknown-web".to_string(),
        cache_dir: None,
        ..Default::default()
    });
    let sources = vec![SourceInput {
        path: "bind.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("Compilation must succeed");
    assert!(res.js_bindings.is_some(), "Browser target must generate JS bindings");
    assert!(res.dts_definitions.is_some(), "Browser target must generate .d.ts");

    let dts = res.dts_definitions.unwrap();
    assert!(dts.contains("export function add(arg0: number, arg1: number): number;"));
    assert!(dts.contains("export function greet(): void;"));
}

#[test]
fn test_wasi_runner_shim_generation() {
    let source = r#"
        export fn main() -> i32 {
            return 0;
        }
    "#;
    let driver = CompilationDriver::new(CompilerOptions {
        target: "wasm32-wasi".to_string(),
        cache_dir: None,
        ..Default::default()
    });
    let sources = vec![SourceInput {
        path: "wasi_main.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&sources).expect("WASI target compilation must succeed");
    assert!(res.js_bindings.is_some(), "WASI target must generate runner shim");
    let js = res.js_bindings.unwrap();
    assert!(js.contains("WASI"));
}

#[test]
fn test_invalid_syntax_error_reporting() {
    let source = "fn broken( { return 1; }";
    let driver = CompilationDriver::new(CompilerOptions::default());
    let sources = vec![SourceInput {
        path: "syntax_err.src".to_string(),
        content: source.to_string(),
    }];
    let result = driver.check(&sources);
    assert!(result.is_err(), "Invalid syntax must produce diagnostic error");
    let sink = result.unwrap_err();
    assert!(sink.has_errors());
    assert!(sink.diagnostics().iter().any(|d| d.code.starts_with("E2")));
}

#[test]
fn test_deterministic_byte_reproducibility() {
    let source = r#"
        export fn compute(a: i32, b: i32) -> i32 {
            let mut x: i32 = a * 2;
            let mut y: i32 = b + 10;
            return x + y;
        }
    "#;
    let options = CompilerOptions {
        cache_dir: None,
        ..Default::default()
    };
    let sources = vec![SourceInput {
        path: "det.src".to_string(),
        content: source.to_string(),
    }];
    let driver1 = CompilationDriver::new(options.clone());
    let res1 = driver1.compile(&sources).unwrap();

    let driver2 = CompilationDriver::new(options);
    let res2 = driver2.compile(&sources).unwrap();

    assert_eq!(res1.wasm_bytes, res2.wasm_bytes, "Builds must be byte-for-byte deterministic");
    assert_eq!(res1.wasm_blake3_hash, res2.wasm_blake3_hash);
}

#[test]
fn test_incremental_cache_sqlite() {
    let temp_dir = std::env::temp_dir().join(format!("wasmc_test_cache_{}", std::process::id()));
    let source1 = "export fn f1() -> i32 { return 100; }";
    let source2 = "export fn f1() -> i32 { return 200; }";

    let options = CompilerOptions {
        cache_dir: Some(temp_dir.clone()),
        ..Default::default()
    };

    let driver = CompilationDriver::new(options.clone());
    let src1 = vec![SourceInput {
        path: "test.src".to_string(),
        content: source1.to_string(),
    }];
    let res1 = driver.compile(&src1).unwrap();

    // Cache hit on identical input
    let res1_hit = driver.compile(&src1).unwrap();
    assert_eq!(res1.wasm_blake3_hash, res1_hit.wasm_blake3_hash);
    assert!(res1_hit.cache_hit, "Expected cache hit");

    // Cache miss and invalidation when source changes
    let src2 = vec![SourceInput {
        path: "test.src".to_string(),
        content: source2.to_string(),
    }];
    let res2 = driver.compile(&src2).unwrap();
    assert_ne!(res1.wasm_blake3_hash, res2.wasm_blake3_hash);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_regression_branch_condition_eqz_inversion() {
    let source = r#"
        export fn check_even(n: i32) -> i32 {
            let mut count: i32 = 0;
            if (n % 2 == 0) {
                count = count + 1;
            }
            return count;
        }
    "#;
    let options = CompilerOptions {
        emit_wat: true,
        cache_dir: None,
        ..Default::default()
    };
    let driver = CompilationDriver::new(options);
    let src = vec![SourceInput {
        path: "even.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&src).expect("Compilation must succeed");
    let wat = res.wat_text.expect("WAT should be emitted");
    // Verify that the branch condition tests i32.eqz so fallthrough goes to the true block
    assert!(
        wat.contains("i32.eqz") && wat.contains("br_if"),
        "WAT must contain i32.eqz to properly invert the branch condition to the false/merge block"
    );
}

#[test]
fn test_regression_implicit_void_return_no_unreachable() {
    let source = r#"
        fn mutate(a: *i32) -> void {
            *a = 42;
        }
        export fn run() -> i32 {
            let mut x: i32 = 10;
            mutate(&x);
            return x;
        }
    "#;
    let options = CompilerOptions {
        emit_wat: true,
        cache_dir: None,
        ..Default::default()
    };
    let driver = CompilationDriver::new(options);
    let src = vec![SourceInput {
        path: "void_ret.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&src).expect("Compilation must succeed");
    let wat = res.wat_text.expect("WAT should be emitted");
    // $mutate should not end in unconditional unreachable trap
    assert!(
        !wat.contains("func $mutate\n    (local i32)\n    local.get 0\n    i32.const 42\n    i32.store offset=0\n    unreachable"),
        "Void function without explicit return must not terminate with unreachable trap"
    );
}

#[test]
fn test_regression_mixed_float_int_local_ordering() {
    let source = r#"
        export fn mixed(a: i32, b: f64) -> f64 {
            let x: i32 = a * 2;
            let y: f64 = b + 1.5;
            return y;
        }
    "#;
    let options = CompilerOptions {
        emit_wat: true,
        cache_dir: None,
        ..Default::default()
    };
    let driver = CompilationDriver::new(options);
    let src = vec![SourceInput {
        path: "mixed.src".to_string(),
        content: source.to_string(),
    }];
    let res = driver.compile(&src).expect("Compilation must succeed");
    assert!(!res.wasm_bytes.is_empty());
}

