use std::path::PathBuf;
use std::process::ExitCode;
use clap::{Parser, Subcommand};
use wasmc_cache::CompilationCache;
use wasmc_daemon::create_router;
use wasmc_driver::{CompilationDriver, CompilerOptions, SourceInput};

#[derive(Parser)]
#[command(
    name = "wasmc",
    author = "wasmc compiler team",
    version = "1.0.0",
    about = "Direct-to-WebAssembly Systems Compiler"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile source file(s) directly into a WebAssembly binary (.wasm)
    Build {
        /// Source file path(s) (.src)
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// Output .wasm file path
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,

        /// Target ABI: wasm32-unknown-web | wasm32-wasi
        #[arg(short = 't', long = "target", default_value = "wasm32-unknown-web")]
        target: String,

        /// Optimization level: 0, 1, 2, 3, s, z
        #[arg(short = 'O', long = "opt-level", default_value = "2")]
        opt_level: String,

        /// Emit debug information
        #[arg(short = 'g', long = "debug")]
        debug: bool,

        /// Emit WebAssembly Text format (.wat) alongside .wasm
        #[arg(long = "emit-wat")]
        emit_wat: bool,

        /// Shadow stack reservation size in bytes (default 1048576 = 1 MiB)
        #[arg(long = "shadow-stack-size", default_value_t = 1048576)]
        shadow_stack_size: u32,

        /// Omit function prologue shadow stack overflow trap check
        #[arg(long = "no-stack-check")]
        no_stack_check: bool,

        /// Omit built-in segregated free-list heap allocator
        #[arg(long = "no-default-allocator")]
        no_default_allocator: bool,

        /// Diagnostic output format: human | json
        #[arg(long = "error-format", default_value = "human")]
        error_format: String,

        /// Path to SQLite incremental build cache directory
        #[arg(long = "cache-dir")]
        cache_dir: Option<PathBuf>,
    },

    /// Run lexical, syntactic, and semantic validation without code generation
    Check {
        /// Source file path(s) (.src)
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// Target ABI: wasm32-unknown-web | wasm32-wasi
        #[arg(short = 't', long = "target", default_value = "wasm32-unknown-web")]
        target: String,

        /// Diagnostic output format: human | json
        #[arg(long = "error-format", default_value = "human")]
        error_format: String,
    },

    /// Start the persistent HTTP/2 REST compiler daemon
    Daemon {
        /// Bind host address
        #[arg(long = "host", default_value = "127.0.0.1")]
        host: String,

        /// Bind port
        #[arg(long = "port", default_value_t = 8080)]
        port: u16,

        /// Path to SQLite incremental build cache directory
        #[arg(long = "cache-dir")]
        cache_dir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            inputs,
            output,
            target,
            opt_level,
            debug,
            emit_wat,
            shadow_stack_size,
            no_stack_check,
            no_default_allocator,
            error_format,
            cache_dir,
        } => {
            let mut sources = Vec::new();
            for path in &inputs {
                match std::fs::read_to_string(path) {
                    Ok(content) => sources.push(SourceInput {
                        path: path.display().to_string(),
                        content,
                    }),
                    Err(e) => {
                        eprintln!("wasmc: error: cannot read file '{}': {}", path.display(), e);
                        return ExitCode::from(2);
                    }
                }
            }

            let options = CompilerOptions {
                target,
                opt_level,
                debug,
                shadow_stack_size,
                emit_wat,
                emit_stack_check: !no_stack_check,
                bundle_allocator: !no_default_allocator,
                cache_dir,
                ..Default::default()
            };

            let driver = CompilationDriver::new(options);

            match driver.compile(&sources) {
                Ok(artifacts) => {
                    let out_path = output.unwrap_or_else(|| {
                        let first = &inputs[0];
                        first.with_extension("wasm")
                    });

                    if let Err(e) = std::fs::write(&out_path, &artifacts.wasm_bytes) {
                        eprintln!("wasmc: error: failed to write output artifact '{}': {}", out_path.display(), e);
                        return ExitCode::from(2);
                    }

                    if let Some(wat) = artifacts.wat_text {
                        let wat_path = out_path.with_extension("wat");
                        let _ = std::fs::write(&wat_path, wat);
                    }

                    if let Some(js) = artifacts.js_bindings {
                        let js_path = out_path.with_extension("js");
                        let _ = std::fs::write(&js_path, js);
                    }

                    if let Some(dts) = artifacts.dts_definitions {
                        let dts_path = out_path.with_extension("d.ts");
                        let _ = std::fs::write(&dts_path, dts);
                    }

                    if error_format == "json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&artifacts.diagnostics).unwrap_or_default()
                        );
                    } else if !artifacts.diagnostics.is_empty() {
                        for diag in &artifacts.diagnostics {
                            eprintln!("{:?}", diag);
                        }
                    }

                    println!(
                        "wasmc: successfully compiled {} ({} bytes, blake3: {}) in {}ms",
                        out_path.display(),
                        artifacts.wasm_size_bytes,
                        &artifacts.wasm_blake3_hash[..16],
                        artifacts.duration_ms
                    );

                    ExitCode::SUCCESS
                }
                Err(sink) => {
                    if error_format == "json" {
                        eprintln!("{}", sink.render_json());
                    } else {
                        for src in &sources {
                            eprint!("{}", sink.render_human(&src.path, &src.content));
                        }
                    }
                    ExitCode::from(1)
                }
            }
        }

        Commands::Check {
            inputs,
            target,
            error_format,
        } => {
            let mut sources = Vec::new();
            for path in &inputs {
                match std::fs::read_to_string(path) {
                    Ok(content) => sources.push(SourceInput {
                        path: path.display().to_string(),
                        content,
                    }),
                    Err(e) => {
                        eprintln!("wasmc: error: cannot read file '{}': {}", path.display(), e);
                        return ExitCode::from(2);
                    }
                }
            }

            let options = CompilerOptions {
                target,
                ..Default::default()
            };

            let driver = CompilationDriver::new(options);

            match driver.check(&sources) {
                Ok(sink) => {
                    if error_format == "json" {
                        println!("{}", sink.render_json());
                    } else if !sink.diagnostics().is_empty() {
                        for src in &sources {
                            print!("{}", sink.render_human(&src.path, &src.content));
                        }
                    }
                    println!("wasmc: check passed successfully for {} input file(s)", sources.len());
                    ExitCode::SUCCESS
                }
                Err(sink) => {
                    if error_format == "json" {
                        eprintln!("{}", sink.render_json());
                    } else {
                        for src in &sources {
                            eprint!("{}", sink.render_human(&src.path, &src.content));
                        }
                    }
                    ExitCode::from(1)
                }
            }
        }

        Commands::Daemon { host, port, cache_dir } => {
            let cache_path = cache_dir.unwrap_or_else(|| PathBuf::from(".wasmc-cache"));
            let cache = CompilationCache::new(cache_path).ok();
            let app = create_router(cache);

            let addr = format!("{}:{}", host, port);
            println!("wasmc: starting compiler daemon on http://{}", addr);

            match tokio::net::TcpListener::bind(&addr).await {
                Ok(listener) => {
                    if let Err(e) = axum::serve(listener, app).await {
                        eprintln!("wasmc daemon error: {}", e);
                        ExitCode::from(2)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("wasmc: failed to bind to {}: {}", addr, e);
                    ExitCode::from(2)
                }
            }
        }
    }
}
