use serde::{Deserialize, Serialize};

pub const NULL_GUARD_BYTES: u32 = 1024;
pub const PAGE_SIZE_BYTES: u32 = 65536;
pub const DEFAULT_SHADOW_STACK_BYTES: u32 = 1048576; // 1 MiB

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryMap {
    pub null_guard_end: u32,
    pub static_data_end: u32,
    pub stack_limit: u32,
    pub stack_pointer_init: u32,
    pub heap_base: u32,
    pub initial_pages: u32,
    pub max_pages: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticDataSegment {
    pub offset: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MemoryPlanner {
    shadow_stack_size: u32,
    emit_stack_check: bool,
    bundle_allocator: bool,
    static_segments: Vec<StaticDataSegment>,
    next_static_offset: u32,
}

impl MemoryPlanner {
    pub fn new(shadow_stack_size: u32, emit_stack_check: bool, bundle_allocator: bool) -> Self {
        Self {
            shadow_stack_size,
            emit_stack_check,
            bundle_allocator,
            static_segments: Vec::new(),
            next_static_offset: NULL_GUARD_BYTES,
        }
    }

    pub fn alloc_static_data(&mut self, data: &[u8], align: u32) -> u32 {
        let align = align.max(1);
        let offset = (self.next_static_offset + align - 1) & !(align - 1);
        self.static_segments.push(StaticDataSegment {
            offset,
            data: data.to_vec(),
        });
        self.next_static_offset = offset + data.len() as u32;
        offset
    }

    pub fn compute_memory_map(&self, min_heap_pages: u32, max_pages: Option<u32>) -> (MemoryMap, Vec<StaticDataSegment>) {
        let static_data_end = ((self.next_static_offset + 15) / 16) * 16;
        let stack_limit = static_data_end;
        let heap_base = ((stack_limit + self.shadow_stack_size + 15) / 16) * 16;
        let stack_pointer_init = heap_base;

        let required_bytes = heap_base + min_heap_pages * PAGE_SIZE_BYTES;
        let initial_pages = (required_bytes + PAGE_SIZE_BYTES - 1) / PAGE_SIZE_BYTES;

        let map = MemoryMap {
            null_guard_end: NULL_GUARD_BYTES,
            static_data_end,
            stack_limit,
            stack_pointer_init,
            heap_base,
            initial_pages: initial_pages.max(17), // At least 17 pages (1MiB stack + data)
            max_pages,
        };

        (map, self.static_segments.clone())
    }

    pub fn should_emit_stack_check(&self) -> bool {
        self.emit_stack_check
    }

    pub fn should_bundle_allocator(&self) -> bool {
        self.bundle_allocator
    }
}
