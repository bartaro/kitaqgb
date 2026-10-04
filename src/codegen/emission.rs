//! Native lowered-IR emission. Rejected IR forms produce diagnostics.
use super::*;
use crate::asm::AddressMode as M;
mod aggregates;
mod analysis;
mod arithmetic;
mod banks;
mod branches;
mod cgb_tiles;
mod countdown;
mod declarations;
mod expressions;
mod externs;
mod inline;
mod intrinsics_cgb;
mod intrinsics_far;
mod intrinsics_geometry;
mod intrinsics_memory;
mod intrinsics_simple;
mod intrinsics_tiles;
mod intrinsics_vram;
mod memory;
mod safety;
mod scaled_arithmetic;
mod scroll;
mod scroll_helpers;
mod shifts;
mod stack_abi;
mod statements;
mod switches;
mod tile_helpers;
mod tile_regions;
mod updates;
use branches::comparison;
use cgb_tiles::TileTarget;

#[derive(Clone, Debug)]
pub struct Output {
    pub lines: Vec<Expr>,
    pub diagnostics: Vec<Diagnostic>,
    pub functions: BTreeMap<String, FunctionPlacement>,
    pub rst_selections: Vec<rst::Selection>,
    pub analysis: super::analysis::Report,
}
#[derive(Clone, Debug)]
pub struct FunctionPlacement {
    pub bank: i32,
    pub fixed: bool,
    pub inline: bool,
    pub prototype: bool,
}
pub fn compile(program: &Expr, options: Options) -> Output {
    let mut emitter = Emitter::new(options);
    emitter.program(program);
    if emitter.env.options.check_bounds
        || emitter.env.options.check_slice_bounds
        || emitter.env.options.check_stack
        || emitter.env.options.check_mem_copy
        || emitter.env.options.check_bank_calls
    {
        emitter.check_trap();
    }
    let rst_selections = rst::select(&emitter.lines, &emitter.env.options.rst);
    emitter.append_tile_helpers();
    emitter.append_scroll_helpers();
    emitter.append_bank_helpers();
    emitter.append_cgb_helper();
    emitter.append_check_helpers();
    for entry in &rst_selections {
        emitter.lines.insert(
            0,
            Expr::new(
                t::RST_MAP,
                vec![entry.vector.into(), entry.target.clone().into()],
            ),
        );
    }
    emitter.finish_analysis(&rst_selections);
    Output {
        lines: emitter.lines,
        diagnostics: emitter.env.diagnostics,
        rst_selections,
        analysis: emitter.analysis,
        functions: emitter
            .env
            .functions
            .into_iter()
            .map(|(name, f)| {
                (
                    name,
                    FunctionPlacement {
                        bank: f.rom_bank,
                        fixed: f.fixed_bank,
                        inline: f.inline,
                        prototype: f.prototype,
                    },
                )
            })
            .collect(),
    }
}
struct Emitter {
    analysis: super::analysis::Report,
    function_order: Vec<String>,
    env: Environment,
    lines: Vec<Expr>,
    next_label: i32,
    return_type: CType,
    current_bank: i32,
    usage: BTreeMap<String, i32>,
    loops: Vec<(Operand, Operand)>,
    breaks: Vec<Operand>,
    thunks: BTreeMap<(i32, String), String>,
    bankswitch_helper: bool,
    stack_base: Option<i32>,
    stack_thunks: BTreeSet<String>,
    cgb_helper: bool,
    far_memcpy: bool,
    far_pointer: bool,
    far_thunks: BTreeMap<String, String>,
    scroll_vars: BTreeMap<String, i32>,
    scroll_split: bool,
    current_function: String,
    last_sret_address: Option<i32>,
    inline_returns: Vec<Operand>,
    inline_destinations: Vec<i32>,
    return_temp_counter: i32,
    unsafe_depth: i32,
    check_trap_label: Option<Operand>,
    assert_panic: bool,
    stack_usage: Option<(i32, i32)>,
}
impl Emitter {
    fn new(options: Options) -> Self {
        Self {
            analysis: super::analysis::Report::default(),
            function_order: Vec::new(),
            env: Environment::new(options),
            lines: Vec::new(),
            next_label: 0,
            return_type: CType::simple(Simple::Void),
            current_bank: 1,
            usage: BTreeMap::new(),
            loops: Vec::new(),
            breaks: Vec::new(),
            thunks: BTreeMap::new(),
            bankswitch_helper: false,
            stack_base: None,
            stack_thunks: BTreeSet::new(),
            cgb_helper: false,
            far_memcpy: false,
            far_pointer: false,
            far_thunks: BTreeMap::new(),
            scroll_vars: BTreeMap::new(),
            scroll_split: false,
            current_function: String::new(),
            last_sret_address: None,
            inline_returns: Vec::new(),
            inline_destinations: Vec::new(),
            return_temp_counter: 0,
            unsafe_depth: 0,
            check_trap_label: None,
            assert_panic: false,
            stack_usage: None,
        }
    }
    fn emit(&mut self, tag: &str, args: Vec<Arg>) {
        self.lines.push(Expr::new(tag, args));
    }
    fn asm(&mut self, mnemonic: &str) {
        self.op(mnemonic, Operand::implicit());
    }
    fn op(&mut self, mnemonic: &str, operand: Operand) {
        if let Some((cur, peak)) = &mut self.stack_usage {
            if mnemonic.starts_with("PUSH_") {
                *cur += 2;
                *peak = (*peak).max(*cur);
            } else if mnemonic.starts_with("POP_") {
                *cur = (*cur - 2).max(0);
            } else if mnemonic == "ADD_SP_IMM" && operand.mode == M::Relative {
                *cur = (*cur - operand.offset).max(0);
                *peak = (*peak).max(*cur);
            } else if mnemonic == "CALL" || mnemonic.starts_with("RST_") {
                *peak = (*peak).max(*cur + 2);
            }
        }
        self.lines.push(Expr::asm(mnemonic, operand));
    }
    fn immediate(&mut self, mnemonic: &str, value: i32) {
        self.op(mnemonic, Operand::integer(value, M::Immediate));
    }
    fn word(&mut self, mnemonic: &str, value: i32) {
        self.op(mnemonic, Operand::integer(value, M::Immediate16));
    }
    fn unique(&mut self, prefix: &str) -> Operand {
        let label = Operand::symbol(format!("{prefix}_{}", self.next_label), M::Absolute);
        self.next_label += 1;
        label
    }
    fn label(&mut self, label: &Operand) {
        self.emit(t::LABEL, vec![label.base.as_deref().unwrap().into()]);
    }
    fn load(&mut self, operand: Operand) {
        self.op(
            if operand.mode == M::HighMem {
                "LDH_A_MEM"
            } else {
                "LD_A_MEM"
            },
            operand,
        );
    }
    fn store(&mut self, operand: Operand) {
        self.op(
            if operand.mode == M::HighMem {
                "LDH_MEM_A"
            } else {
                "LD_MEM_A"
            },
            operand,
        );
    }
    fn size(&mut self, e: &Expr) -> i32 {
        let ty = self.env.type_of(e);
        self.env.size(e, &ty)
    }
    fn unsupported(&mut self, e: &Expr, path: &str) {
        let message = match path {
            "byte expression" => "Expression too complex for CompileIntoA",
            "word expression" => "Expression too complex for CompileIntoHL",
            "variable shift" => "Variable shift not supported",
            "assignment expression lvalue" | "complex assignment lvalue" => {
                "Assignment expression LHS not supported"
            }
            "lvalue address" => "Address-of form not supported",
            _ => path,
        };
        self.env.error(&e.source, format!("Not Implemented: {message}"));
    }
    fn operand(&mut self, expr: &Expr) -> Option<Operand> {
        if expr.is(t::INTEGER) {
            return Some(Operand::integer(expr.int(1)?, M::Immediate));
        }
        if !expr.is(t::NAME) {
            return None;
        }
        let name = expr.text(1)?;
        let symbol = self.env.find_required(expr, name)?;
        if symbol.ty.is_array() {
            return Some(if symbol.tag == SymbolTag::ReadonlyData {
                Operand::symbol(name, M::Immediate16)
            } else {
                Operand::integer(symbol.value, M::Immediate16)
            });
        }
        match symbol.tag {
            SymbolTag::ReadonlyData => Some(Operand::symbol(name, M::Absolute)),
            SymbolTag::Global | SymbolTag::Local => Some(memory_operand(symbol.value)),
            SymbolTag::Constant => Some(Operand::integer(symbol.value, M::Immediate)),
            SymbolTag::StackParam => None,
        }
    }
    fn extend_a(&mut self, signed: bool) {
        self.asm("LD_L_A");
        if signed {
            self.asm("RLCA");
            self.asm("SBC_A");
            self.asm("LD_H_A");
        } else {
            self.immediate("LD_H_IMM", 0);
        }
    }
    fn sign_extend(&mut self, expr: &Expr) -> bool {
        matches!(self.env.type_of(expr).kind, Kind::Simple(Simple::Int8))
    }
    fn usage(&mut self, e: &Expr, weight: i32) {
        if e.is(t::NAME) {
            *self.usage.entry(e.text(1).unwrap().into()).or_default() += weight;
            return;
        }
        if e.is(t::FOR) {
            for i in 1..=4 {
                self.usage(child(e, i), if i == 1 { weight } else { weight * 4 });
            }
            return;
        }
        for e in e.children() {
            self.usage(e, weight);
        }
    }
}
