//! Native, host-independent KITAQGB components.
//! Implementation and executed validation are recorded in PORT_STATUS.md.
#![forbid(unsafe_code)]

pub mod analysis_reports;
pub mod asm;
pub mod asm_info;
pub mod assembler;
pub mod assembly_json;
pub mod build_helpers;
pub mod cache;
pub mod cli;
pub mod codegen;
pub mod compiler_api;
pub mod ctype;
pub mod dead_stripper;
pub mod debug_exporter;
pub mod debug_tools;
mod diagnostic_help;
pub mod diagnostics;
pub mod disassembler;
pub mod drivers;
pub mod expr;
pub mod hash;
pub mod io;
pub mod ir_json;
pub mod json;
pub mod lowerer;
pub mod metadata;
pub mod minimizer;
pub mod observation;
pub mod optimizer;
pub mod palettes;
pub mod parser;
pub mod pipeline;
mod preprocessor;
pub mod recipe;
pub mod reports;
pub mod repro;
pub mod rom_header;
pub mod tags;
pub mod token;
pub mod tokenizer;
pub mod vibe_tools;
pub mod workflow;
pub mod zx0;
pub mod vblank_patch;
