use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use serde::{Deserialize, Serialize};

use wasmc_ast::AstPool;
use wasmc_bindings::BindingGenerator;
use wasmc_cache::CompilationCache;
use wasmc_diagnostics::{Diagnostic, DiagnosticSink};
use wasmc_hir::HirLowerer;
use wasmc_lexer::{Lexer, SymbolInterner};
use wasmc_memory::{MemoryMap, MemoryPlanner, DEFAULT_SHADOW_STACK_BYTES};
use wasmc_mir::lower_hir_to_mir;
use wasmc_optimizer::{OptLevel, Optimizer};
use wasmc_parser::Parser;
use wasmc_semantics::SemanticAnalyzer;
use wasmc_stackifier::Stackifier;
use wasmc_wasm::{
    BinaryEmitter, CodeGenerator, DataSegment, ExportEntry, ExportKind, FuncType, GlobalEntry,
    MemoryLimits, TableLimits, WasmOpcode, WasmValType, WatPrinter,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInput {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerOptions {
    pub target: String, // "wasm32-unknown-web" | "wasm32-wasi"
    pub opt_level: String, // "O0" | "O1" | "O2" | "O3" | "Os" | "Oz"
    pub debug: bool,
    pub shadow_stack_size: u32,
    pub emit_wat: bool,
    pub emit_stack_check: bool,
    pub bundle_allocator: bool,
    pub enable_exceptions: bool,
    pub enable_tail_calls: bool,
    pub cache_dir: Option<PathBuf>,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            target: "wasm32-unknown-web".to_string(),
            opt_level: "O2".to_string(),
            debug: false,
            shadow_stack_size: DEFAULT_SHADOW_STACK_BYTES,
            emit_wat: false,
            emit_stack_check: true,
            bundle_allocator: true,
            enable_exceptions: true,
            enable_tail_calls: true,
            cache_dir: Some(PathBuf::from(".wasmc-cache")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationArtifacts {
    pub wasm_bytes: Vec<u8>,
    pub wasm_blake3_hash: String,
    pub wasm_size_bytes: usize,
    pub wat_text: Option<String>,
    pub js_bindings: Option<String>,
    pub dts_definitions: Option<String>,
    pub memory_map: MemoryMap,
    pub diagnostics: Vec<Diagnostic>,
    pub duration_ms: u128,
    pub cache_hit: bool,
}

pub struct CompilationDriver {
    options: CompilerOptions,
    cache: Option<CompilationCache>,
}

impl CompilationDriver {
    pub fn new(options: CompilerOptions) -> Self {
        let cache = options
            .cache_dir
            .as_ref()
            .and_then(|dir| CompilationCache::new(dir).ok());
        Self { options, cache }
    }

    pub fn check(&self, sources: &[SourceInput]) -> Result<DiagnosticSink, DiagnosticSink> {
        let mut sink = DiagnosticSink::new();
        let mut interner = SymbolInterner::new();
        let mut pool = AstPool::new();

        for (file_id, src) in sources.iter().enumerate() {
            let mut lexer = Lexer::new(&src.content, file_id as u32, &mut interner);
            let tokens = lexer.tokenize_all(&mut sink);
            let mut parser = Parser::new(tokens, &mut sink);
            parser.parse_module(&mut pool);
        }

        if sink.has_errors() {
            return Err(sink);
        }

        let analyzer = SemanticAnalyzer::new(&pool, &mut sink);
        analyzer.analyze()?;

        Ok(sink)
    }

    pub fn compile(
        &self,
        sources: &[SourceInput],
    ) -> Result<CompilationArtifacts, DiagnosticSink> {
        let start_time = Instant::now();
        let mut sink = DiagnosticSink::new();

        // Single unit cache lookup check
        if sources.len() == 1 {
            let src = &sources[0];
            let src_hash = CompilationCache::compute_source_hash(&src.content);
            let flags_hash = CompilationCache::compute_flags_hash_extended(
                &self.options.target,
                &self.options.opt_level,
                self.options.debug,
                self.options.shadow_stack_size,
                self.options.emit_stack_check,
                self.options.bundle_allocator,
            );

            if let Some(ref cache) = self.cache {
                if let Ok(Some(cached_wasm)) = cache.lookup_unit(&src.path, &src_hash, &flags_hash) {
                    let hash = blake3::hash(&cached_wasm).to_hex().to_string();
                    let mem_planner = MemoryPlanner::new(
                        self.options.shadow_stack_size,
                        self.options.emit_stack_check,
                        self.options.bundle_allocator,
                    );
                    let (mem_map, _) = mem_planner.compute_memory_map(1, None);

                    return Ok(CompilationArtifacts {
                        wasm_bytes: cached_wasm.clone(),
                        wasm_blake3_hash: hash,
                        wasm_size_bytes: cached_wasm.len(),
                        wat_text: None,
                        js_bindings: None,
                        dts_definitions: None,
                        memory_map: mem_map,
                        diagnostics: Vec::new(),
                        duration_ms: start_time.elapsed().as_millis(),
                        cache_hit: true,
                    });
                }
            }
        }

        // 1. Lexical and Syntactic Analysis
        let mut interner = SymbolInterner::new();
        let mut pool = AstPool::new();

        for (file_id, src) in sources.iter().enumerate() {
            let mut lexer = Lexer::new(&src.content, file_id as u32, &mut interner);
            let tokens = lexer.tokenize_all(&mut sink);
            let mut parser = Parser::new(tokens, &mut sink);
            parser.parse_module(&mut pool);
        }

        if sink.has_errors() {
            return Err(sink);
        }

        // 2. Semantic Analysis & Escape Analysis
        let analyzer = SemanticAnalyzer::new(&pool, &mut sink);
        let sem_model = match analyzer.analyze() {
            Ok(m) => m,
            Err(_) => return Err(sink),
        };

        // 3. HIR Lowering
        let hir_lowerer = HirLowerer::new(&pool, &sem_model);
        let hir_module = hir_lowerer.lower_module();

        // 4. MIR SSA Construction
        let mir_module = lower_hir_to_mir(hir_module);

        // 5. Optimization
        let opt_level = OptLevel::from_str(&self.options.opt_level).unwrap_or(OptLevel::O2);
        let optimizer = Optimizer::new(opt_level);
        let optimized_mir = optimizer.optimize_module(mir_module);

        // 6. Stackifier (Control-Flow Structuring)
        let structured_module = Stackifier::stackify_module(optimized_mir);

        // 7. Linear Memory Planning
        let mem_planner = MemoryPlanner::new(
            self.options.shadow_stack_size,
            self.options.emit_stack_check,
            self.options.bundle_allocator,
        );

        // String literals placed in static data
        for (idx, node) in pool.top_level_decls.iter().enumerate() {
            let _ = (idx, node);
        }

        let (memory_map, data_segs) = mem_planner.compute_memory_map(1, None);

        // 8. Wasm Code Generation & Binary Emission
        let mut emitter = BinaryEmitter::new();

        // Table definition (for indirect function calls)
        emitter.set_table(TableLimits { min: 10, max: None });

        // Memory definition
        emitter.set_memory(MemoryLimits {
            min: memory_map.initial_pages,
            max: memory_map.max_pages,
        });

        // Global #0: __stack_pointer (mutable i32) initialized to __heap_base
        emitter.add_global(GlobalEntry {
            val_type: WasmValType::I32,
            mutable: true,
            init_insts: vec![WasmOpcode::I32Const(memory_map.stack_pointer_init as i32)],
        });

        // Index mapping for functions
        let mut func_indices: HashMap<wasmc_lexer::SymbolId, u32> = HashMap::new();
        let mut func_type_indices: HashMap<wasmc_lexer::SymbolId, u32> = HashMap::new();

        let mut func_idx_counter = 0u32;

        // Process function imports first so they occupy indices 0..N-1
        for &decl_idx in &pool.top_level_decls {
            if let wasmc_ast::AstNode::ImportDecl { module, field, func_name, params, return_type } = pool.get(decl_idx) {
                let mod_str = interner.resolve(*module).unwrap_or("env").to_string();
                let field_str = interner.resolve(*field).unwrap_or("").to_string();

                let p_types: Vec<WasmValType> = params.iter().map(|p| {
                    match pool.get(p.type_node) {
                        wasmc_ast::AstNode::TypePrimitive(wasmc_ast::PrimType::I64) | wasmc_ast::AstNode::TypePrimitive(wasmc_ast::PrimType::U64) => WasmValType::I64,
                        wasmc_ast::AstNode::TypePrimitive(wasmc_ast::PrimType::F32) => WasmValType::F32,
                        wasmc_ast::AstNode::TypePrimitive(wasmc_ast::PrimType::F64) => WasmValType::F64,
                        _ => WasmValType::I32,
                    }
                }).collect();

                let r_types: Vec<WasmValType> = if let Some(r) = return_type {
                    match pool.get(*r) {
                        wasmc_ast::AstNode::TypePrimitive(wasmc_ast::PrimType::Void) => Vec::new(),
                        wasmc_ast::AstNode::TypePrimitive(wasmc_ast::PrimType::I64) | wasmc_ast::AstNode::TypePrimitive(wasmc_ast::PrimType::U64) => vec![WasmValType::I64],
                        wasmc_ast::AstNode::TypePrimitive(wasmc_ast::PrimType::F32) => vec![WasmValType::F32],
                        wasmc_ast::AstNode::TypePrimitive(wasmc_ast::PrimType::F64) => vec![WasmValType::F64],
                        _ => vec![WasmValType::I32],
                    }
                } else {
                    Vec::new()
                };

                let ft = FuncType { params: p_types, results: r_types };
                let t_idx = emitter.add_type(ft);
                emitter.add_import(wasmc_wasm::ImportEntry {
                    module: mod_str,
                    name: field_str,
                    kind: wasmc_wasm::ImportKind::Func(t_idx),
                });

                func_indices.insert(*func_name, func_idx_counter);
                func_type_indices.insert(*func_name, t_idx);
                func_idx_counter += 1;
            }
        }

        for f in &structured_module.functions {
            let params = f.params.iter().map(|(_, ty)| Self::mir_to_wasm_val(*ty)).collect();
            let results = if f.return_type != wasmc_mir::MirType::Void {
                vec![Self::mir_to_wasm_val(f.return_type)]
            } else {
                Vec::new()
            };
            let ft = FuncType { params, results };
            let t_idx = emitter.add_type(ft);

            func_indices.insert(f.name, func_idx_counter);
            func_type_indices.insert(f.name, t_idx);
            func_idx_counter += 1;
        }

        let codegen = CodeGenerator::new(
            &interner,
            &func_indices,
            &func_type_indices,
            0, // global #0 is stack pointer
            memory_map.stack_limit,
            self.options.emit_stack_check,
        );

        let mut compiled_functions = Vec::new();
        for f in &structured_module.functions {
            let compiled = codegen.compile_function(f, &mut sink);
            compiled_functions.push(compiled.clone());
            emitter.add_function(compiled);
        }

        // Exports
        for f in &structured_module.functions {
            let name_str = interner.resolve(f.name).unwrap_or("anon");
            let f_idx = func_indices[&f.name];
            emitter.add_export(ExportEntry {
                name: name_str.to_string(),
                kind: ExportKind::Func,
                index: f_idx,
            });
        }

        // Export memory
        emitter.add_export(ExportEntry {
            name: "memory".to_string(),
            kind: ExportKind::Memory,
            index: 0,
        });

        // Add static data segments
        for seg in data_segs {
            emitter.add_data_segment(DataSegment {
                memory_index: 0,
                offset: seg.offset,
                bytes: seg.data,
            });
        }

        // WAT text output if requested
        let wat_text = if self.options.emit_wat {
            let printer = WatPrinter::new(&interner);
            let mut exports_list = Vec::new();
            for f in &structured_module.functions {
                let name_str = interner.resolve(f.name).unwrap_or("anon");
                let f_idx = func_indices[&f.name];
                exports_list.push(ExportEntry {
                    name: name_str.to_string(),
                    kind: ExportKind::Func,
                    index: f_idx,
                });
            }
            exports_list.push(ExportEntry {
                name: "memory".to_string(),
                kind: ExportKind::Memory,
                index: 0,
            });
            Some(printer.print_module(
                &compiled_functions,
                emitter.types(),
                emitter.imports(),
                Some(MemoryLimits {
                    min: memory_map.initial_pages,
                    max: memory_map.max_pages,
                }),
                &[GlobalEntry {
                    val_type: WasmValType::I32,
                    mutable: true,
                    init_insts: vec![WasmOpcode::I32Const(memory_map.stack_pointer_init as i32)],
                }],
                &exports_list,
            ))
        } else {
            None
        };

        // Binary serialization
        let wasm_bytes = emitter.serialize();
        let wasm_size = wasm_bytes.len();
        let wasm_hash = blake3::hash(&wasm_bytes).to_hex().to_string();

        // Host bindings generation (JS/TS)
        let binding_gen = BindingGenerator::new(&interner);
        let exported_syms: Vec<wasmc_semantics::SymbolEntry> = sem_model
            .functions
            .values()
            .cloned()
            .collect();

        let (js_bindings, dts_definitions) = match self.options.target.as_str() {
            "wasm32-wasi" => {
                let shim = binding_gen.generate_wasi_start_shim("main");
                (Some(shim), None)
            }
            _ => {
                let host_bindings = binding_gen.generate_web_bindings("wasmc_module", &exported_syms);
                (Some(host_bindings.js_glue), Some(host_bindings.ts_definitions))
            }
        };

        // Update incremental cache
        if sources.len() == 1 {
            let src = &sources[0];
            let src_hash = CompilationCache::compute_source_hash(&src.content);
            let flags_hash = CompilationCache::compute_flags_hash_extended(
                &self.options.target,
                &self.options.opt_level,
                self.options.debug,
                self.options.shadow_stack_size,
                self.options.emit_stack_check,
                self.options.bundle_allocator,
            );
            if let Some(ref cache) = self.cache {
                let _ = cache.store_unit(&src.path, &src_hash, &flags_hash, &wasm_bytes);
            }
        }

        let duration_ms = start_time.elapsed().as_millis();

        Ok(CompilationArtifacts {
            wasm_bytes,
            wasm_blake3_hash: wasm_hash,
            wasm_size_bytes: wasm_size,
            wat_text,
            js_bindings,
            dts_definitions,
            memory_map,
            diagnostics: sink.into_diagnostics(),
            duration_ms,
            cache_hit: false,
        })
    }

    fn mir_to_wasm_val(ty: wasmc_mir::MirType) -> WasmValType {
        match ty {
            wasmc_mir::MirType::I32 | wasmc_mir::MirType::Void => WasmValType::I32,
            wasmc_mir::MirType::I64 => WasmValType::I64,
            wasmc_mir::MirType::F32 => WasmValType::F32,
            wasmc_mir::MirType::F64 => WasmValType::F64,
        }
    }
}
