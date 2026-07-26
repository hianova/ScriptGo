#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
pub mod ast;
pub mod codegen;
pub mod ir;
pub mod lexer;
pub mod optimizer;
pub mod parser;

#[cfg(test)]
mod lexer_tests;
#[cfg(test)]
mod parser_tests;
#[cfg(test)]
mod m2_stress_tests;

