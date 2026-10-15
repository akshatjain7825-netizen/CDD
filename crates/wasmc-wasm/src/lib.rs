pub mod binary;
pub mod codegen;
pub mod leb128;
pub mod types;
pub mod wat;

pub use binary::BinaryEmitter;
pub use codegen::{CodeGenerator, CompiledFunction};
pub use leb128::{Leb128Codec, Leb128Error};
pub use types::*;
pub use wat::WatPrinter;
