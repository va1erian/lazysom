// Library target — re-exports all modules for use by integration tests.
#![allow(non_local_definitions)]  // gc_derive's Finalize macro
#![allow(unsafe_op_in_unsafe_fn)] // custom_trace! macro under Rust 2024

pub mod lexer;
pub mod ast;
pub mod parser;
pub mod object;
pub mod universe;
pub mod interpreter;
pub mod primitives;
pub mod bytecode;
pub mod compiler;
pub mod bytecode_interpreter;
pub mod gui;
