use wasmc_lexer::SymbolInterner;
use crate::codegen::CompiledFunction;
use crate::types::*;

pub struct WatPrinter<'a> {
    interner: &'a SymbolInterner,
}

impl<'a> WatPrinter<'a> {
    pub fn new(interner: &'a SymbolInterner) -> Self {
        Self { interner }
    }

    pub fn print_module(
        &self,
        functions: &[CompiledFunction],
        types: &[FuncType],
        imports: &[ImportEntry],
        memory: Option<MemoryLimits>,
        globals: &[GlobalEntry],
        exports: &[ExportEntry],
    ) -> String {
        let mut out = String::from("(module\n");

        // Types
        for (i, ty) in types.iter().enumerate() {
            out.push_str(&format!("  (type (;{};) (func", i));
            if !ty.params.is_empty() {
                out.push_str(" (param");
                for p in &ty.params {
                    out.push_str(&format!(" {}", Self::type_name(p)));
                }
                out.push(')');
            }
            if !ty.results.is_empty() {
                out.push_str(" (result");
                for r in &ty.results {
                    out.push_str(&format!(" {}", Self::type_name(r)));
                }
                out.push(')');
            }
            out.push_str("))\n");
        }

        // Imports
        for (i, imp) in imports.iter().enumerate() {
            match &imp.kind {
                ImportKind::Func(t_idx) => {
                    out.push_str(&format!("  (import \"{}\" \"{}\" (func (;{};) (type {})))\n", imp.module, imp.name, i, t_idx));
                }
                _ => {}
            }
        }

        // Memory
        if let Some(mem) = memory {
            if let Some(max) = mem.max {
                out.push_str(&format!("  (memory (export \"memory\") {} {})\n", mem.min, max));
            } else {
                out.push_str(&format!("  (memory (export \"memory\") {})\n", mem.min));
            }
        }

        // Globals
        for (i, g) in globals.iter().enumerate() {
            let mut_str = if g.mutable { "mut " } else { "" };
            out.push_str(&format!("  (global (;{};) ({}{})", i, mut_str, Self::type_name(&g.val_type)));
            out.push_str(" (i32.const 0))\n");
        }

        // Functions
        for func in functions {
            let name = self.interner.resolve(func.name).unwrap_or("anon");
            let fty = &types[func.type_index as usize];
            out.push_str(&format!("  (func ${} (type {})", name, func.type_index));

            for p in &fty.params {
                out.push_str(&format!(" (param {})", Self::type_name(p)));
            }
            for r in &fty.results {
                out.push_str(&format!(" (result {})", Self::type_name(r)));
            }
            out.push('\n');

            // Locals
            for (count, ty) in &func.locals {
                for _ in 0..*count {
                    out.push_str(&format!("    (local {})\n", Self::type_name(ty)));
                }
            }

            // Body
            for opc in &func.body_opcodes {
                out.push_str(&format!("    {}\n", Self::print_opcode(opc)));
            }

            out.push_str("  )\n");
        }

        // Exports
        for exp in exports {
            match exp.kind {
                ExportKind::Func => {
                    out.push_str(&format!("  (export \"{}\" (func {}))\n", exp.name, exp.index));
                }
                ExportKind::Memory => {
                    out.push_str(&format!("  (export \"{}\" (memory {}))\n", exp.name, exp.index));
                }
                ExportKind::Global => {
                    out.push_str(&format!("  (export \"{}\" (global {}))\n", exp.name, exp.index));
                }
                ExportKind::Table => {
                    out.push_str(&format!("  (export \"{}\" (table {}))\n", exp.name, exp.index));
                }
            }
        }

        out.push_str(")\n");
        out
    }

    fn type_name(ty: &WasmValType) -> &'static str {
        match ty {
            WasmValType::I32 => "i32",
            WasmValType::I64 => "i64",
            WasmValType::F32 => "f32",
            WasmValType::F64 => "f64",
            WasmValType::V128 => "v128",
            WasmValType::FuncRef => "funcref",
            WasmValType::ExternRef => "externref",
        }
    }

    fn print_opcode(opc: &WasmOpcode) -> String {
        match opc {
            WasmOpcode::Unreachable => "unreachable".to_string(),
            WasmOpcode::Nop => "nop".to_string(),
            WasmOpcode::Block(_) => "block".to_string(),
            WasmOpcode::Loop(_) => "loop".to_string(),
            WasmOpcode::If(_) => "if".to_string(),
            WasmOpcode::Else => "else".to_string(),
            WasmOpcode::End => "end".to_string(),
            WasmOpcode::Br(d) => format!("br {}", d),
            WasmOpcode::BrIf(d) => format!("br_if {}", d),
            WasmOpcode::Return => "return".to_string(),
            WasmOpcode::Call(idx) => format!("call {}", idx),
            WasmOpcode::CallIndirect { type_idx, .. } => format!("call_indirect (type {})", type_idx),
            WasmOpcode::ReturnCall(idx) => format!("return_call {}", idx),
            WasmOpcode::ReturnCallIndirect { type_idx, .. } => format!("return_call_indirect (type {})", type_idx),
            WasmOpcode::Drop => "drop".to_string(),
            WasmOpcode::Select => "select".to_string(),
            WasmOpcode::LocalGet(idx) => format!("local.get {}", idx),
            WasmOpcode::LocalSet(idx) => format!("local.set {}", idx),
            WasmOpcode::LocalTee(idx) => format!("local.tee {}", idx),
            WasmOpcode::GlobalGet(idx) => format!("global.get {}", idx),
            WasmOpcode::GlobalSet(idx) => format!("global.set {}", idx),
            WasmOpcode::I32Load { offset, .. } => format!("i32.load offset={}", offset),
            WasmOpcode::I64Load { offset, .. } => format!("i64.load offset={}", offset),
            WasmOpcode::F32Load { offset, .. } => format!("f32.load offset={}", offset),
            WasmOpcode::F64Load { offset, .. } => format!("f64.load offset={}", offset),
            WasmOpcode::I32Store { offset, .. } => format!("i32.store offset={}", offset),
            WasmOpcode::I64Store { offset, .. } => format!("i64.store offset={}", offset),
            WasmOpcode::F32Store { offset, .. } => format!("f32.store offset={}", offset),
            WasmOpcode::F64Store { offset, .. } => format!("f64.store offset={}", offset),
            WasmOpcode::MemorySize => "memory.size".to_string(),
            WasmOpcode::MemoryGrow => "memory.grow".to_string(),
            WasmOpcode::I32Const(n) => format!("i32.const {}", n),
            WasmOpcode::I64Const(n) => format!("i64.const {}", n),
            WasmOpcode::F32Const(f) => format!("f32.const {}", f),
            WasmOpcode::F64Const(f) => format!("f64.const {}", f),
            WasmOpcode::I32Eqz => "i32.eqz".to_string(),
            WasmOpcode::I32Eq => "i32.eq".to_string(),
            WasmOpcode::I32Ne => "i32.ne".to_string(),
            WasmOpcode::I32LtS => "i32.lt_s".to_string(),
            WasmOpcode::I32LtU => "i32.lt_u".to_string(),
            WasmOpcode::I32GtS => "i32.gt_s".to_string(),
            WasmOpcode::I32GtU => "i32.gt_u".to_string(),
            WasmOpcode::I32LeS => "i32.le_s".to_string(),
            WasmOpcode::I32LeU => "i32.le_u".to_string(),
            WasmOpcode::I32GeS => "i32.ge_s".to_string(),
            WasmOpcode::I32GeU => "i32.ge_u".to_string(),
            WasmOpcode::I32Add => "i32.add".to_string(),
            WasmOpcode::I32Sub => "i32.sub".to_string(),
            WasmOpcode::I32Mul => "i32.mul".to_string(),
            WasmOpcode::I32DivS => "i32.div_s".to_string(),
            WasmOpcode::I32DivU => "i32.div_u".to_string(),
            WasmOpcode::I32RemS => "i32.rem_s".to_string(),
            WasmOpcode::I32RemU => "i32.rem_u".to_string(),
            WasmOpcode::I32And => "i32.and".to_string(),
            WasmOpcode::I32Or => "i32.or".to_string(),
            WasmOpcode::I32Xor => "i32.xor".to_string(),
            WasmOpcode::I32Shl => "i32.shl".to_string(),
            WasmOpcode::I32ShrS => "i32.shr_s".to_string(),
            WasmOpcode::I32ShrU => "i32.shr_u".to_string(),
            WasmOpcode::F64Add => "f64.add".to_string(),
            WasmOpcode::F64Sub => "f64.sub".to_string(),
            WasmOpcode::F64Mul => "f64.mul".to_string(),
            WasmOpcode::F64Div => "f64.div".to_string(),
            WasmOpcode::F64Eq => "f64.eq".to_string(),
            WasmOpcode::F64Ne => "f64.ne".to_string(),
            WasmOpcode::F64Lt => "f64.lt".to_string(),
            WasmOpcode::F64Le => "f64.le".to_string(),
            WasmOpcode::F64Gt => "f64.gt".to_string(),
            WasmOpcode::F64Ge => "f64.ge".to_string(),
            WasmOpcode::F64Neg => "f64.neg".to_string(),
            WasmOpcode::I32TruncSatF64S => "i32.trunc_sat_f64_s".to_string(),
            WasmOpcode::F64ConvertI32S => "f64.convert_i32_s".to_string(),
            WasmOpcode::TryTable { .. } => "try_table".to_string(),
            WasmOpcode::Throw(tag) => format!("throw {}", tag),
            _ => "nop".to_string(),
        }
    }
}
