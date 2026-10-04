//! Recompile bank-dependent calls after assembly has placed overflowing functions.
use crate::{assembler, codegen, expr::Expr, optimizer, tokenizer::Severity};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
pub struct Build {
    pub generated: codegen::emission::Output,
    pub assembled: Option<assembler::Output>,
    pub relayout_passes: usize,
    pub codegen_time: Duration,
    pub assemble_time: Duration,
}
pub fn compile_lowered(
    tree: &Expr,
    mut options: codegen::Options,
    asm: &assembler::Options,
) -> Result<Build, String> {
    let mut seen = BTreeSet::new();
    seen.insert(options.function_banks.clone());
    let mut passes = 0;
    let mut codegen_time = Duration::ZERO;
    let mut assemble_time = Duration::ZERO;
    loop {
        let start = Instant::now();
        let generated = codegen::emission::compile(tree, options.clone());
        if generated
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
        {
            codegen_time += start.elapsed();
            return Ok(Build {
                generated,
                assembled: None,
                relayout_passes: passes,
                codegen_time,
                assemble_time,
            });
        }
        let optimized = optimizer::optimize(&generated.lines, options.opt_level);
        codegen_time += start.elapsed();
        let start = Instant::now();
        let assembled = assembler::assemble(&optimized.lines, asm)?;
        assemble_time += start.elapsed();
        let mut relocations = Vec::new();
        for f in &assembled.report.functions {
            if let Some(plan) = generated.functions.get(&f.name) {
                if plan.inline || plan.prototype || plan.bank == f.bank {
                    continue;
                }
                if plan.fixed {
                    return Err(format!(
                        "fixed-bank function spilled to a different bank: {} requested b{} actual b{}",
                        f.name, plan.bank, f.bank
                    ));
                }
                relocations.push((f.name.clone(), f.bank));
            }
        }
        if relocations.is_empty() {
            return Ok(Build {
                generated,
                assembled: Some(assembled),
                relayout_passes: passes,
                codegen_time,
                assemble_time,
            });
        }
        passes += 1;
        let limit = (generated.functions.len() * 2 + 2).clamp(16, 512);
        if passes > limit {
            return Err(format!(
                "function bank layout did not stabilize after {passes} passes"
            ));
        }
        // Preserve prior overrides, including functions that matched their actual bank this pass.
        options.function_banks.extend(relocations);
        if !seen.insert(options.function_banks.clone()) {
            return Err(format!(
                "function bank layout entered a relocation cycle after {passes} passes"
            ));
        }
    }
}
