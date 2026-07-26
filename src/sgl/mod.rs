#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
pub mod assembler;
pub mod host_handlers;
pub mod instruction;
pub mod macros;
pub mod simd_ops;
pub mod ui_engine;
pub mod vm;

pub mod io;
pub mod net;

pub use assembler::*;
pub use host_handlers::*;
pub use instruction::*;
pub use io::SglIoRegisterExt;
pub use macros::*;
pub use net::SglNetRegisterExt;
pub use simd_ops::*;
pub use ui_engine::*;
pub use vm::*;

#[cfg(test)]
pub mod vm_tests;
#[cfg(test)]
pub mod sgl_macro_simd_stress_tests;
