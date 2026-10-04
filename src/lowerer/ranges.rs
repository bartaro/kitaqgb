use super::*;
pub(super) fn scalar_info(ty: &CType) -> Option<(u32, bool)> {
    Some(match &ty.kind {
        Kind::Pointer(_) | Kind::Enum(_) => (16, false),
        Kind::Simple(Simple::UInt8) => (8, false),
        Kind::Simple(Simple::Int8) => (8, true),
        Kind::Simple(Simple::UInt16) => (16, false),
        Kind::Simple(Simple::Int16) => (16, true),
        _ => return None,
    })
}
fn bounds(bits: u32, signed: bool) -> [i32; 2] {
    if signed {
        [-(1 << (bits - 1)), (1 << (bits - 1)) - 1]
    } else {
        [0, (1 << bits) - 1]
    }
}
fn clamp(n: i64) -> i32 {
    n.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}
fn side_effect_free(e: &Expr) -> bool {
    if matches!(
        e.tag(),
        Some(
            t::CALL
                | t::ASSIGN
                | t::ASSIGN_MODIFY
                | t::PRE_INCREMENT
                | t::POST_INCREMENT
                | t::PRE_DECREMENT
                | t::POST_DECREMENT
        )
    ) {
        return false;
    }
    e.args.iter().all(|a| match a {
        Arg::Expr(e) => side_effect_free(e),
        Arg::Exprs(es) => es.iter().all(side_effect_free),
        _ => true,
    })
}
impl Context<'_> {
    pub(super) fn get_range(&self, name: &str) -> Option<[i32; 2]> {
        self.overrides
            .iter()
            .rev()
            .find_map(|m| m.get(name))
            .or_else(|| self.local_ranges.get(name))
            .or_else(|| self.param_ranges.get(name))
            .or_else(|| self.global.ranges.get(name))
            .copied()
    }
    pub(super) fn interval(&mut self, e: &Expr) -> Option<[i32; 2]> {
        let tag = e.tag().unwrap_or("");
        match tag {
            t::INTEGER => e.int(1).map(|n| [n, n]),
            t::NAME => self
                .get_range(e.text(1)?)
                .or_else(|| self.global.try_eval(e).map(|n| [n, n])),
            t::CAST => {
                let mut range = self.interval(child(e, 2))?;
                if let Some((bits, signed)) = scalar_info(e.ty(1)?) {
                    let b = bounds(bits, signed);
                    range = [range[0].max(b[0]), range[1].min(b[1])]
                }
                Some(range)
            }
            t::CONDITIONAL => {
                let a = self.interval(child(e, 2))?;
                let b = self.interval(child(e, 3))?;
                Some([a[0].min(b[0]), a[1].max(b[1])])
            }
            t::ADD | t::SUBTRACT => {
                let a = self.interval(child(e, 1))?;
                let b = self.interval(child(e, 2))?;
                Some(if tag == t::ADD {
                    [
                        clamp(a[0] as i64 + b[0] as i64),
                        clamp(a[1] as i64 + b[1] as i64),
                    ]
                } else {
                    [
                        clamp(a[0] as i64 - b[1] as i64),
                        clamp(a[1] as i64 - b[0] as i64),
                    ]
                })
            }
            t::SHIFT_LEFT | t::SHIFT_RIGHT => {
                let rhs = child(e, 2);
                let shift = rhs
                    .int(1)
                    .filter(|_| rhs.is(t::INTEGER))
                    .filter(|n| (0..=15).contains(n))?;
                let a = self.interval(child(e, 1))?;
                Some(if tag == t::SHIFT_LEFT {
                    [clamp((a[0] as i64) << shift), clamp((a[1] as i64) << shift)]
                } else {
                    [a[0] >> shift, a[1] >> shift]
                })
            }
            t::MODULUS => {
                let rhs = child(e, 2);
                let n = rhs
                    .int(1)
                    .filter(|_| rhs.is(t::INTEGER))
                    .filter(|n| *n > 0)?;
                let a = self.interval(child(e, 1))?;
                if a[0] >= 0 { Some([0, n - 1]) } else { None }
            }
            t::BITWISE_AND => {
                let left = child(e, 1);
                let right = child(e, 2);
                let l = left.int(1).filter(|_| left.is(t::INTEGER));
                let r = right.int(1).filter(|_| right.is(t::INTEGER));
                if let (Some(a), Some(b)) = (l, r) {
                    return Some([a & b, a & b]);
                }
                let (mask, other) = if let Some(n) = l.filter(|n| *n >= 0) {
                    (n, right)
                } else if let Some(n) = r.filter(|n| *n >= 0) {
                    (n, left)
                } else {
                    return None;
                };
                let a = self.interval(other)?;
                if a[0] >= 0 {
                    Some([0, mask.min(a[1])])
                } else {
                    None
                }
            }
            _ => self.global.try_eval(e).map(|n| [n, n]),
        }
    }
    pub(super) fn non_negative(&mut self, e: &Expr) -> bool {
        if e.is(t::INTEGER) && e.int(1).is_some_and(|n| n >= 0) {
            return true;
        }
        if self.interval(e).is_some_and(|r| r[0] >= 0) {
            return true;
        }
        let ty = self.infer(e).without_const();
        ty.is_pointer() || ty.is_enum() || ty.is_unsigned()
    }
    pub(super) fn condition_from_ranges(&mut self, e: &Expr) -> Option<i32> {
        if !side_effect_free(e) {
            return None;
        }
        match e.tag() {
            Some(t::INTEGER) => return e.int(1).map(|n| i32::from(n != 0)),
            Some(t::LOGICAL_NOT) => {
                return self
                    .condition_from_ranges(child(e, 1))
                    .map(|n| i32::from(n == 0));
            }
            Some(t::LOGICAL_AND) => {
                return self.condition_from_ranges(child(e, 1)).and_then(|n| {
                    if n == 0 {
                        Some(0)
                    } else {
                        self.condition_from_ranges(child(e, 2))
                    }
                });
            }
            Some(t::LOGICAL_OR) => {
                return self.condition_from_ranges(child(e, 1)).and_then(|n| {
                    if n != 0 {
                        Some(1)
                    } else {
                        self.condition_from_ranges(child(e, 2))
                    }
                });
            }
            Some(tag) if comparison(tag) => {
                if let (Some(a), Some(b)) = (self.interval(child(e, 1)), self.interval(child(e, 2)))
                {
                    let disjoint = a[1] < b[0] || a[0] > b[1];
                    let equal = a[0] == a[1] && b[0] == b[1] && a[0] == b[0];
                    let (yes, no) = match tag {
                        t::EQUAL => (equal, disjoint),
                        t::NOT_EQUAL => (disjoint, equal),
                        t::LESS_THAN => (a[1] < b[0], a[0] >= b[1]),
                        t::LESS_THAN_OR_EQUAL => (a[1] <= b[0], a[0] > b[1]),
                        t::GREATER_THAN => (a[0] > b[1], a[1] <= b[0]),
                        _ => (a[0] >= b[1], a[1] < b[0]),
                    };
                    if yes {
                        return Some(1);
                    }
                    if no {
                        return Some(0);
                    }
                }
            }
            _ => {}
        }
        let a = self.interval(e)?;
        if a == [0, 0] {
            Some(0)
        } else if a[0] > 0 || a[1] < 0 {
            Some(1)
        } else {
            None
        }
    }
    pub(super) fn index_range(&mut self, origin: &Expr, array: &Expr, mut index: Expr) -> Expr {
        if let Some([min, max]) = self.interval(&index) {
            let ty = self.infer(array);
            let len = match &ty.kind {
                Kind::Array(_, n) if *n >= 0 => Some(*n),
                Kind::ArrayExpression(_, n) => self.global.try_eval(n),
                _ => None,
            };
            if let Some(len) = len.filter(|n| *n > 0) {
                if min < 0 || max >= len {
                    let message = if index.is(t::NAME)
                        && self.get_range(index.text(1).unwrap()).is_some()
                    {
                        format!(
                            "__range({min},{max}) index '{}' may go out of bounds for array length {len}",
                            index.text(1).unwrap()
                        )
                    } else {
                        format!(
                            "index range [{min},{max}] may go out of bounds for array length {len}"
                        )
                    };
                    self.warning(&origin.source, message);
                } else if min >= 0 && max < len && max <= 65535 && !index.is(t::CAST) {
                    let mut ty = CType::simple(if max <= 255 {
                        Simple::UInt8
                    } else {
                        Simple::UInt16
                    });
                    ty.is_safe_index = true;
                    let index_source = index.source.clone();
                    index = node(
                        t::CAST,
                        vec![Arg::from(ty), Arg::from(index)],
                        &index_source,
                    )
                }
            }
        }
        if index.is(t::NAME) {
            let name = index.text(1).unwrap();
            let ty = self.infer(&index);
            if ty.is_integer() && !ty.is_enum() && !ty.is_safe_index {
                self.warning(&origin.source,format!("array index '{name}' has type {ty} which is not __safe_index; consider '__safe_index' or cast explicitly to silence"))
            }
        }
        index
    }
    pub(super) fn refine(&mut self, e: &Expr, yes: bool) -> BTreeMap<String, [i32; 2]> {
        let mut map = BTreeMap::new();
        self.add_refinement(e, yes, &mut map);
        map
    }
    fn add_refinement(&mut self, e: &Expr, yes: bool, map: &mut BTreeMap<String, [i32; 2]>) {
        let mut op = e.tag().unwrap_or("");
        if op == t::LOGICAL_NOT {
            self.add_refinement(child(e, 1), !yes, map);
            return;
        }
        if (op == t::LOGICAL_AND && yes) || (op == t::LOGICAL_OR && !yes) {
            self.add_refinement(child(e, 1), yes, map);
            self.add_refinement(child(e, 2), yes, map);
            return;
        }
        if !comparison(op) || op == t::NOT_EQUAL {
            return;
        }
        let a = child(e, 1);
        let b = child(e, 2);
        let (name, n) = if a.is(t::NAME) {
            let Some(n) = self.global.try_eval(b) else {
                return;
            };
            (a.text(1).unwrap(), n)
        } else if b.is(t::NAME) {
            let Some(n) = self.global.try_eval(a) else {
                return;
            };
            op = match op {
                t::LESS_THAN => t::GREATER_THAN,
                t::LESS_THAN_OR_EQUAL => t::GREATER_THAN_OR_EQUAL,
                t::GREATER_THAN => t::LESS_THAN,
                t::GREATER_THAN_OR_EQUAL => t::LESS_THAN_OR_EQUAL,
                _ => op,
            };
            (b.text(1).unwrap(), n)
        } else {
            return;
        };
        let Some(mut range) = self.get_range(name) else {
            return;
        };
        if let Some(current) = map.get(name) {
            range = *current
        }
        if !yes {
            op = match op {
                t::LESS_THAN => t::GREATER_THAN_OR_EQUAL,
                t::LESS_THAN_OR_EQUAL => t::GREATER_THAN,
                t::GREATER_THAN => t::LESS_THAN_OR_EQUAL,
                t::GREATER_THAN_OR_EQUAL => t::LESS_THAN,
                _ => return,
            }
        }
        match op {
            t::GREATER_THAN => range[0] = range[0].max(n.wrapping_add(1)),
            t::GREATER_THAN_OR_EQUAL => range[0] = range[0].max(n),
            t::LESS_THAN => range[1] = range[1].min(n.wrapping_sub(1)),
            t::LESS_THAN_OR_EQUAL => range[1] = range[1].min(n),
            t::EQUAL => {
                range[0] = range[0].max(n);
                range[1] = range[1].min(n)
            }
            _ => return,
        }
        if range[0] <= range[1] {
            map.insert(name.into(), range);
        }
    }
    pub(super) fn narrowing(
        &mut self,
        value: &Expr,
        src: &CType,
        dst: &CType,
        position: &Position,
        context: &str,
    ) {
        if value.is(t::CAST) {
            return;
        }
        let (Some((sb, ss)), Some((db, ds))) = (scalar_info(src), scalar_info(dst)) else {
            return;
        };
        if !(sb > db || (sb >= db && ss && !ds) || (sb == db && !ss && ds)) {
            return;
        }
        if self.global.try_eval(value).is_some_and(|n| {
            let r = bounds(db, ds);
            n >= r[0] && n <= r[1]
        }) {
            return;
        }
        self.warning(
            position,
            format!(
                "implicit narrowing in {}: {src} -> {dst}; cast explicitly to silence",
                if context.is_empty() {
                    "conversion"
                } else {
                    context
                }
            ),
        );
    }
}
