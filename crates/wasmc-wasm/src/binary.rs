use crate::codegen::CompiledFunction;
use crate::leb128::Leb128Codec;
use crate::types::*;

pub struct BinaryEmitter {
    types: Vec<FuncType>,
    imports: Vec<ImportEntry>,
    func_type_indices: Vec<u32>,
    table: Option<TableLimits>,
    memory: Option<MemoryLimits>,
    globals: Vec<GlobalEntry>,
    exports: Vec<ExportEntry>,
    start_func: Option<u32>,
    elements: Vec<ElementSegment>,
    functions: Vec<CompiledFunction>,
    data_segments: Vec<DataSegment>,
    custom_sections: Vec<CustomSection>,
}

impl BinaryEmitter {
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            imports: Vec::new(),
            func_type_indices: Vec::new(),
            table: None,
            memory: None,
            globals: Vec::new(),
            exports: Vec::new(),
            start_func: None,
            elements: Vec::new(),
            functions: Vec::new(),
            data_segments: Vec::new(),
            custom_sections: Vec::new(),
        }
    }

    pub fn add_type(&mut self, ft: FuncType) -> u32 {
        if let Some(pos) = self.types.iter().position(|t| t == &ft) {
            return pos as u32;
        }
        let idx = self.types.len() as u32;
        self.types.push(ft);
        idx
    }

    pub fn types(&self) -> &[FuncType] {
        &self.types
    }

    pub fn add_import(&mut self, import: ImportEntry) {
        self.imports.push(import);
    }

    pub fn imports(&self) -> &[ImportEntry] {
        &self.imports
    }

    pub fn set_table(&mut self, limits: TableLimits) {
        self.table = Some(limits);
    }

    pub fn set_memory(&mut self, limits: MemoryLimits) {
        self.memory = Some(limits);
    }

    pub fn add_global(&mut self, global: GlobalEntry) {
        self.globals.push(global);
    }

    pub fn add_export(&mut self, export: ExportEntry) {
        self.exports.push(export);
    }

    pub fn set_start(&mut self, func_idx: u32) {
        self.start_func = Some(func_idx);
    }

    pub fn add_element(&mut self, elem: ElementSegment) {
        self.elements.push(elem);
    }

    pub fn add_function(&mut self, func: CompiledFunction) {
        self.func_type_indices.push(func.type_index);
        self.functions.push(func);
    }

    pub fn add_data_segment(&mut self, seg: DataSegment) {
        self.data_segments.push(seg);
    }

    pub fn add_custom_section(&mut self, sec: CustomSection) {
        self.custom_sections.push(sec);
    }

    pub fn serialize(self) -> Vec<u8> {
        let mut out = Vec::new();

        // 8-byte Preamble: Magic number \0asm and Version 1
        out.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]);
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

        // Section 1: Type
        if !self.types.is_empty() {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, self.types.len() as u32);
            for ty in &self.types {
                sec_payload.push(0x60); // func type constructor
                Leb128Codec::encode_u32(&mut sec_payload, ty.params.len() as u32);
                for p in &ty.params {
                    sec_payload.push(*p as u8);
                }
                Leb128Codec::encode_u32(&mut sec_payload, ty.results.len() as u32);
                for r in &ty.results {
                    sec_payload.push(*r as u8);
                }
            }
            Self::emit_section(&mut out, 1, &sec_payload);
        }

        // Section 2: Import
        if !self.imports.is_empty() {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, self.imports.len() as u32);
            for imp in &self.imports {
                Self::emit_string(&mut sec_payload, &imp.module);
                Self::emit_string(&mut sec_payload, &imp.name);
                match &imp.kind {
                    ImportKind::Func(t_idx) => {
                        sec_payload.push(0x00);
                        Leb128Codec::encode_u32(&mut sec_payload, *t_idx);
                    }
                    ImportKind::Table { element_type, min, max } => {
                        sec_payload.push(0x01);
                        sec_payload.push(*element_type as u8);
                        Self::emit_limits(&mut sec_payload, *min, *max);
                    }
                    ImportKind::Memory { min, max } => {
                        sec_payload.push(0x02);
                        Self::emit_limits(&mut sec_payload, *min, *max);
                    }
                    ImportKind::Global { val_type, mutable } => {
                        sec_payload.push(0x03);
                        sec_payload.push(*val_type as u8);
                        sec_payload.push(if *mutable { 0x01 } else { 0x00 });
                    }
                }
            }
            Self::emit_section(&mut out, 2, &sec_payload);
        }

        // Section 3: Function
        if !self.func_type_indices.is_empty() {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, self.func_type_indices.len() as u32);
            for t_idx in &self.func_type_indices {
                Leb128Codec::encode_u32(&mut sec_payload, *t_idx);
            }
            Self::emit_section(&mut out, 3, &sec_payload);
        }

        // Section 4: Table
        if let Some(tbl) = self.table {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, 1); // 1 table
            sec_payload.push(0x70); // funcref
            Self::emit_limits(&mut sec_payload, tbl.min, tbl.max);
            Self::emit_section(&mut out, 4, &sec_payload);
        }

        // Section 5: Memory
        if let Some(mem) = self.memory {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, 1); // 1 memory
            Self::emit_limits(&mut sec_payload, mem.min, mem.max);
            Self::emit_section(&mut out, 5, &sec_payload);
        }

        // Section 6: Global
        if !self.globals.is_empty() {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, self.globals.len() as u32);
            for g in &self.globals {
                sec_payload.push(g.val_type as u8);
                sec_payload.push(if g.mutable { 0x01 } else { 0x00 });
                for inst in &g.init_insts {
                    Self::emit_opcode(&mut sec_payload, inst);
                }
                sec_payload.push(0x0B); // end
            }
            Self::emit_section(&mut out, 6, &sec_payload);
        }

        // Section 7: Export
        if !self.exports.is_empty() {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, self.exports.len() as u32);
            for exp in &self.exports {
                Self::emit_string(&mut sec_payload, &exp.name);
                sec_payload.push(exp.kind as u8);
                Leb128Codec::encode_u32(&mut sec_payload, exp.index);
            }
            Self::emit_section(&mut out, 7, &sec_payload);
        }

        // Section 8: Start
        if let Some(start_idx) = self.start_func {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, start_idx);
            Self::emit_section(&mut out, 8, &sec_payload);
        }

        // Section 9: Element
        if !self.elements.is_empty() {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, self.elements.len() as u32);
            for elem in &self.elements {
                sec_payload.push(0x00); // active table 0
                sec_payload.push(0x41); // i32.const
                Leb128Codec::encode_i32(&mut sec_payload, elem.offset as i32);
                sec_payload.push(0x0B); // end
                Leb128Codec::encode_u32(&mut sec_payload, elem.func_indices.len() as u32);
                for f_idx in &elem.func_indices {
                    Leb128Codec::encode_u32(&mut sec_payload, *f_idx);
                }
            }
            Self::emit_section(&mut out, 9, &sec_payload);
        }

        // Section 10: Code
        if !self.functions.is_empty() {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, self.functions.len() as u32);
            for func in &self.functions {
                let mut body = Vec::new();
                // Local declarations
                Leb128Codec::encode_u32(&mut body, func.locals.len() as u32);
                for (count, ty) in &func.locals {
                    Leb128Codec::encode_u32(&mut body, *count);
                    body.push(*ty as u8);
                }
                // Bytecode opcodes
                for opc in &func.body_opcodes {
                    Self::emit_opcode(&mut body, opc);
                }

                Leb128Codec::encode_u32(&mut sec_payload, body.len() as u32);
                sec_payload.extend_from_slice(&body);
            }
            Self::emit_section(&mut out, 10, &sec_payload);
        }

        // Section 11: Data
        if !self.data_segments.is_empty() {
            let mut sec_payload = Vec::new();
            Leb128Codec::encode_u32(&mut sec_payload, self.data_segments.len() as u32);
            for seg in &self.data_segments {
                sec_payload.push(0x00); // active segment memory 0
                sec_payload.push(0x41); // i32.const
                Leb128Codec::encode_i32(&mut sec_payload, seg.offset as i32);
                sec_payload.push(0x0B); // end
                Leb128Codec::encode_u32(&mut sec_payload, seg.bytes.len() as u32);
                sec_payload.extend_from_slice(&seg.bytes);
            }
            Self::emit_section(&mut out, 11, &sec_payload);
        }

        // Section 0: Custom Sections (appended after standard sections per Section 5.3)
        for custom in &self.custom_sections {
            let mut sec_payload = Vec::new();
            Self::emit_string(&mut sec_payload, &custom.name);
            sec_payload.extend_from_slice(&custom.payload);
            Self::emit_section(&mut out, 0, &sec_payload);
        }

        out
    }

    fn emit_section(out: &mut Vec<u8>, sec_id: u8, payload: &[u8]) {
        out.push(sec_id);
        Leb128Codec::encode_u32(out, payload.len() as u32);
        out.extend_from_slice(payload);
    }

    fn emit_string(out: &mut Vec<u8>, s: &str) {
        Leb128Codec::encode_u32(out, s.len() as u32);
        out.extend_from_slice(s.as_bytes());
    }

    fn emit_limits(out: &mut Vec<u8>, min: u32, max: Option<u32>) {
        if let Some(m) = max {
            out.push(0x01); // has max
            Leb128Codec::encode_u32(out, min);
            Leb128Codec::encode_u32(out, m);
        } else {
            out.push(0x00); // min only
            Leb128Codec::encode_u32(out, min);
        }
    }

    fn emit_opcode(out: &mut Vec<u8>, opc: &WasmOpcode) {
        match opc {
            WasmOpcode::Unreachable => out.push(0x00),
            WasmOpcode::Nop => out.push(0x01),
            WasmOpcode::Block(bt) => {
                out.push(0x02);
                Self::emit_block_type(out, bt);
            }
            WasmOpcode::Loop(bt) => {
                out.push(0x03);
                Self::emit_block_type(out, bt);
            }
            WasmOpcode::If(bt) => {
                out.push(0x04);
                Self::emit_block_type(out, bt);
            }
            WasmOpcode::Else => out.push(0x05),
            WasmOpcode::End => out.push(0x0B),
            WasmOpcode::Br(d) => {
                out.push(0x0C);
                Leb128Codec::encode_u32(out, *d);
            }
            WasmOpcode::BrIf(d) => {
                out.push(0x0D);
                Leb128Codec::encode_u32(out, *d);
            }
            WasmOpcode::Return => out.push(0x0F),
            WasmOpcode::Call(idx) => {
                out.push(0x10);
                Leb128Codec::encode_u32(out, *idx);
            }
            WasmOpcode::CallIndirect { type_idx, table_idx } => {
                out.push(0x11);
                Leb128Codec::encode_u32(out, *type_idx);
                Leb128Codec::encode_u32(out, *table_idx);
            }
            WasmOpcode::ReturnCall(idx) => {
                out.push(0x12);
                Leb128Codec::encode_u32(out, *idx);
            }
            WasmOpcode::ReturnCallIndirect { type_idx, table_idx } => {
                out.push(0x13);
                Leb128Codec::encode_u32(out, *type_idx);
                Leb128Codec::encode_u32(out, *table_idx);
            }
            WasmOpcode::Drop => out.push(0x1A),
            WasmOpcode::Select => out.push(0x1B),

            WasmOpcode::LocalGet(idx) => {
                out.push(0x20);
                Leb128Codec::encode_u32(out, *idx);
            }
            WasmOpcode::LocalSet(idx) => {
                out.push(0x21);
                Leb128Codec::encode_u32(out, *idx);
            }
            WasmOpcode::LocalTee(idx) => {
                out.push(0x22);
                Leb128Codec::encode_u32(out, *idx);
            }
            WasmOpcode::GlobalGet(idx) => {
                out.push(0x23);
                Leb128Codec::encode_u32(out, *idx);
            }
            WasmOpcode::GlobalSet(idx) => {
                out.push(0x24);
                Leb128Codec::encode_u32(out, *idx);
            }

            WasmOpcode::I32Load { align, offset } => {
                out.push(0x28);
                Leb128Codec::encode_u32(out, align.trailing_zeros());
                Leb128Codec::encode_u32(out, *offset);
            }
            WasmOpcode::I64Load { align, offset } => {
                out.push(0x29);
                Leb128Codec::encode_u32(out, align.trailing_zeros());
                Leb128Codec::encode_u32(out, *offset);
            }
            WasmOpcode::F32Load { align, offset } => {
                out.push(0x2A);
                Leb128Codec::encode_u32(out, align.trailing_zeros());
                Leb128Codec::encode_u32(out, *offset);
            }
            WasmOpcode::F64Load { align, offset } => {
                out.push(0x2B);
                Leb128Codec::encode_u32(out, align.trailing_zeros());
                Leb128Codec::encode_u32(out, *offset);
            }

            WasmOpcode::I32Store { align, offset } => {
                out.push(0x36);
                Leb128Codec::encode_u32(out, align.trailing_zeros());
                Leb128Codec::encode_u32(out, *offset);
            }
            WasmOpcode::I64Store { align, offset } => {
                out.push(0x37);
                Leb128Codec::encode_u32(out, align.trailing_zeros());
                Leb128Codec::encode_u32(out, *offset);
            }
            WasmOpcode::F32Store { align, offset } => {
                out.push(0x38);
                Leb128Codec::encode_u32(out, align.trailing_zeros());
                Leb128Codec::encode_u32(out, *offset);
            }
            WasmOpcode::F64Store { align, offset } => {
                out.push(0x39);
                Leb128Codec::encode_u32(out, align.trailing_zeros());
                Leb128Codec::encode_u32(out, *offset);
            }

            WasmOpcode::MemorySize => {
                out.push(0x3F);
                out.push(0x00);
            }
            WasmOpcode::MemoryGrow => {
                out.push(0x40);
                out.push(0x00);
            }

            WasmOpcode::I32Const(val) => {
                out.push(0x41);
                Leb128Codec::encode_i32(out, *val);
            }
            WasmOpcode::I64Const(val) => {
                out.push(0x42);
                Leb128Codec::encode_i64(out, *val);
            }
            WasmOpcode::F32Const(val) => {
                out.push(0x43);
                out.extend_from_slice(&val.to_le_bytes());
            }
            WasmOpcode::F64Const(val) => {
                out.push(0x44);
                out.extend_from_slice(&val.to_le_bytes());
            }

            WasmOpcode::I32Eqz => out.push(0x45),
            WasmOpcode::I32Eq => out.push(0x46),
            WasmOpcode::I32Ne => out.push(0x47),
            WasmOpcode::I32LtS => out.push(0x48),
            WasmOpcode::I32LtU => out.push(0x49),
            WasmOpcode::I32GtS => out.push(0x4A),
            WasmOpcode::I32GtU => out.push(0x4B),
            WasmOpcode::I32LeS => out.push(0x4C),
            WasmOpcode::I32LeU => out.push(0x4D),
            WasmOpcode::I32GeS => out.push(0x4E),
            WasmOpcode::I32GeU => out.push(0x4F),

            WasmOpcode::I64Eqz => out.push(0x50),
            WasmOpcode::I64Eq => out.push(0x51),
            WasmOpcode::I64Ne => out.push(0x52),
            WasmOpcode::I64LtS => out.push(0x53),
            WasmOpcode::I64LtU => out.push(0x54),
            WasmOpcode::I64GtS => out.push(0x55),
            WasmOpcode::I64GtU => out.push(0x56),
            WasmOpcode::I64LeS => out.push(0x57),
            WasmOpcode::I64LeU => out.push(0x58),
            WasmOpcode::I64GeS => out.push(0x59),
            WasmOpcode::I64GeU => out.push(0x5A),

            WasmOpcode::F32Eq => out.push(0x5B),
            WasmOpcode::F32Ne => out.push(0x5C),
            WasmOpcode::F32Lt => out.push(0x5D),
            WasmOpcode::F32Gt => out.push(0x5E),
            WasmOpcode::F32Le => out.push(0x5F),
            WasmOpcode::F32Ge => out.push(0x60),

            WasmOpcode::F64Eq => out.push(0x61),
            WasmOpcode::F64Ne => out.push(0x62),
            WasmOpcode::F64Lt => out.push(0x63),
            WasmOpcode::F64Gt => out.push(0x64),
            WasmOpcode::F64Le => out.push(0x65),
            WasmOpcode::F64Ge => out.push(0x66),

            WasmOpcode::I32Add => out.push(0x6A),
            WasmOpcode::I32Sub => out.push(0x6B),
            WasmOpcode::I32Mul => out.push(0x6C),
            WasmOpcode::I32DivS => out.push(0x6D),
            WasmOpcode::I32DivU => out.push(0x6E),
            WasmOpcode::I32RemS => out.push(0x6F),
            WasmOpcode::I32RemU => out.push(0x70),
            WasmOpcode::I32And => out.push(0x71),
            WasmOpcode::I32Or => out.push(0x72),
            WasmOpcode::I32Xor => out.push(0x73),
            WasmOpcode::I32Shl => out.push(0x74),
            WasmOpcode::I32ShrS => out.push(0x75),
            WasmOpcode::I32ShrU => out.push(0x76),

            WasmOpcode::I64Add => out.push(0x7C),
            WasmOpcode::I64Sub => out.push(0x7D),
            WasmOpcode::I64Mul => out.push(0x7E),
            WasmOpcode::I64DivS => out.push(0x7F),
            WasmOpcode::I64DivU => out.push(0x80),
            WasmOpcode::I64RemS => out.push(0x81),
            WasmOpcode::I64RemU => out.push(0x82),
            WasmOpcode::I64And => out.push(0x83),
            WasmOpcode::I64Or => out.push(0x84),
            WasmOpcode::I64Xor => out.push(0x85),
            WasmOpcode::I64Shl => out.push(0x86),
            WasmOpcode::I64ShrS => out.push(0x87),
            WasmOpcode::I64ShrU => out.push(0x88),

            WasmOpcode::F32Neg => out.push(0x8C),
            WasmOpcode::F32Add => out.push(0x92),
            WasmOpcode::F32Sub => out.push(0x93),
            WasmOpcode::F32Mul => out.push(0x94),
            WasmOpcode::F32Div => out.push(0x95),

            WasmOpcode::F64Neg => out.push(0x9A),
            WasmOpcode::F64Add => out.push(0xA0),
            WasmOpcode::F64Sub => out.push(0xA1),
            WasmOpcode::F64Mul => out.push(0xA2),
            WasmOpcode::F64Div => out.push(0xA3),

            WasmOpcode::I32TruncSatF32S => {
                out.push(0xFC);
                Leb128Codec::encode_u32(out, 0x00);
            }
            WasmOpcode::I32TruncSatF64S => {
                out.push(0xFC);
                Leb128Codec::encode_u32(out, 0x02);
            }
            WasmOpcode::I64TruncSatF32S => {
                out.push(0xFC);
                Leb128Codec::encode_u32(out, 0x04);
            }
            WasmOpcode::I64TruncSatF64S => {
                out.push(0xFC);
                Leb128Codec::encode_u32(out, 0x06);
            }
            WasmOpcode::F32ConvertI32S => out.push(0xB2),
            WasmOpcode::F64ConvertI32S => out.push(0xB7),
            WasmOpcode::F64ConvertI64S => out.push(0xB9),

            WasmOpcode::TryTable { block_ty, catches } => {
                out.push(0x1F);
                Self::emit_block_type(out, block_ty);
                Leb128Codec::encode_u32(out, catches.len() as u32);
                for c in catches {
                    match c {
                        CatchClause::Catch { tag_idx, label_depth } => {
                            out.push(0x00);
                            Leb128Codec::encode_u32(out, *tag_idx);
                            Leb128Codec::encode_u32(out, *label_depth);
                        }
                        CatchClause::CatchRef { tag_idx, label_depth } => {
                            out.push(0x01);
                            Leb128Codec::encode_u32(out, *tag_idx);
                            Leb128Codec::encode_u32(out, *label_depth);
                        }
                        CatchClause::CatchAll { label_depth } => {
                            out.push(0x02);
                            Leb128Codec::encode_u32(out, *label_depth);
                        }
                        CatchClause::CatchAllRef { label_depth } => {
                            out.push(0x03);
                            Leb128Codec::encode_u32(out, *label_depth);
                        }
                    }
                }
            }
            WasmOpcode::Throw(tag) => {
                out.push(0x08);
                Leb128Codec::encode_u32(out, *tag);
            }
        }
    }

    fn emit_block_type(out: &mut Vec<u8>, bt: &BlockType) {
        match bt {
            BlockType::Empty => out.push(0x40),
            BlockType::Value(vt) => out.push(*vt as u8),
        }
    }
}
