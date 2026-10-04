//! Integer expression evaluator for #if, including unevaluated short-circuit branches.
use crate::expr::Position;
use crate::token::{Kind, Token};
use crate::tokenizer::{Diagnostic, Macro, Severity};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn evaluate(
    tokens: &[Token],
    macros: &BTreeMap<String, Macro>,
    position: &Position,
    diagnostics: &mut Vec<Diagnostic>,
) -> i64 {
    Evaluator {
        tokens,
        index: 0,
        macros,
        position,
        diagnostics,
        active: BTreeSet::new(),
        depth: 0,
    }
    .expression(1, true)
}
struct Evaluator<'a> {
    tokens: &'a [Token],
    index: usize,
    macros: &'a BTreeMap<String, Macro>,
    position: &'a Position,
    diagnostics: &'a mut Vec<Diagnostic>,
    active: BTreeSet<String>,
    depth: usize,
}
impl Evaluator<'_> {
    fn warning(&mut self, message: &str) {
        self.diagnostics.push(Diagnostic::new(
            Severity::Warning,
            self.position.clone(),
            message.into(),
            0,
        ));
    }
    fn peek(&self) -> Kind {
        self.tokens.get(self.index).map_or(Kind::EOF, |t| t.kind)
    }
    fn take(&mut self, kind: Kind) -> bool {
        if self.peek() == kind {
            self.index += 1;
            true
        } else {
            false
        }
    }
    fn primary(&mut self, execute: bool) -> i64 {
        if self.take(Kind::LPAREN) {
            let value = self.expression(1, execute);
            if !self.take(Kind::RPAREN) {
                self.warning("missing ')' in #if expression");
            }
            return value;
        }
        let Some(t) = self.tokens.get(self.index).cloned() else {
            return 0;
        };
        self.index += 1;
        if t.kind == Kind::INT {
            return i64::from(t.integer);
        }
        if t.kind != Kind::NAME {
            return 0;
        }
        let name = t.name.unwrap_or_default();
        if name == "defined" {
            let paren = self.take(Kind::LPAREN);
            let macro_name = if self.peek() == Kind::NAME {
                let n = self.tokens[self.index].name.clone();
                self.index += 1;
                n
            } else {
                self.warning(if paren {
                    "malformed defined(...) in #if expression"
                } else {
                    "malformed defined usage in #if expression"
                });
                None
            };
            if paren && !self.take(Kind::RPAREN) {
                self.warning("missing ')' after defined(...)");
            }
            return i64::from(
                macro_name
                    .as_ref()
                    .is_some_and(|n| self.macros.contains_key(n)),
            );
        }
        if self.active.contains(&name) {
            return 0;
        }
        if let Some(m) = self.macros.get(&name).filter(|m| m.parameters.is_none()) {
            if self.depth >= 32 {
                self.warning("macro expansion depth exceeded (32) in #if expression");
                return 0;
            }
            let mut active = self.active.clone();
            active.insert(name);
            let mut next = Evaluator {
                tokens: &m.replacement,
                index: 0,
                macros: self.macros,
                position: self.position,
                diagnostics: self.diagnostics,
                active,
                depth: self.depth + 1,
            };
            return next.expression(1, execute);
        }
        0
    }
    fn unary(&mut self, execute: bool) -> i64 {
        if self.take(Kind::PLUS) {
            self.unary(execute)
        } else if self.take(Kind::MINUS) {
            self.unary(execute).wrapping_neg()
        } else if self.take(Kind::LOGICAL_NOT) {
            i64::from(self.unary(execute) == 0)
        } else if self.take(Kind::TILDE) {
            !self.unary(execute)
        } else {
            self.primary(execute)
        }
    }
    fn expression(&mut self, min_precedence: u8, execute: bool) -> i64 {
        let mut value = self.unary(execute);
        loop {
            let op = self.peek();
            let precedence = match op {
                Kind::LOGICAL_OR => 1,
                Kind::LOGICAL_AND => 2,
                Kind::PIPE => 3,
                Kind::CARET => 4,
                Kind::AMPERSAND => 5,
                Kind::DOUBLE_EQUAL | Kind::NOT_EQUAL => 6,
                Kind::LESS_THAN
                | Kind::LESS_THAN_OR_EQUAL
                | Kind::GREATER_THAN
                | Kind::GREATER_THAN_OR_EQUAL => 7,
                Kind::SHIFT_LEFT | Kind::SHIFT_RIGHT => 8,
                Kind::PLUS | Kind::MINUS => 9,
                Kind::STAR | Kind::SLASH | Kind::PERCENT => 10,
                _ => 0,
            };
            if precedence < min_precedence {
                break;
            }
            self.index += 1;
            let evaluate_right = execute
                && match op {
                    Kind::LOGICAL_AND => value != 0,
                    Kind::LOGICAL_OR => value == 0,
                    _ => true,
                };
            let right = self.expression(precedence + 1, evaluate_right);
            value = match op {
                Kind::PLUS => value.wrapping_add(right),
                Kind::MINUS => value.wrapping_sub(right),
                Kind::STAR => value.wrapping_mul(right),
                Kind::SLASH | Kind::PERCENT => {
                    if right == 0 {
                        if execute {
                            self.warning(if op == Kind::SLASH {
                                "division by zero in #if expression"
                            } else {
                                "modulo by zero in #if expression"
                            });
                        }
                        0
                    } else if !execute {
                        0
                    } else if op == Kind::SLASH {
                        value.wrapping_div(right)
                    } else {
                        value.wrapping_rem(right)
                    }
                }
                Kind::SHIFT_LEFT => value.wrapping_shl((right & 63) as u32),
                Kind::SHIFT_RIGHT => value.wrapping_shr((right & 63) as u32),
                Kind::LESS_THAN => i64::from(value < right),
                Kind::LESS_THAN_OR_EQUAL => i64::from(value <= right),
                Kind::GREATER_THAN => i64::from(value > right),
                Kind::GREATER_THAN_OR_EQUAL => i64::from(value >= right),
                Kind::DOUBLE_EQUAL => i64::from(value == right),
                Kind::NOT_EQUAL => i64::from(value != right),
                Kind::AMPERSAND => value & right,
                Kind::CARET => value ^ right,
                Kind::PIPE => value | right,
                Kind::LOGICAL_AND => i64::from(value != 0 && right != 0),
                Kind::LOGICAL_OR => i64::from(value != 0 || right != 0),
                _ => unreachable!(),
            };
        }
        value
    }
}
