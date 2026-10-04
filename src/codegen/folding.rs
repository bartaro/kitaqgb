use super::*;

impl Environment {
    /// Fold using target integer semantics while preserving casts and lvalues.
    pub fn fold(&mut self, expr: &Expr) -> Expr {
        if let Some(value) = self.try_constant(expr) {
            let integer =
                Expr::new(t::INTEGER, vec![value.value.into()]).with_source(expr.source.clone());
            return if expr.is(t::CAST) {
                Expr::new(t::CAST, vec![expr.args[1].clone(), integer.into()])
                    .with_source(expr.source.clone())
            } else {
                integer
            };
        }
        let mut result = expr.clone();
        let indexes: Vec<usize> = match expr.tag().unwrap_or("") {
            t::ADDRESS_OF => {
                let sub = child(expr, 1);
                if sub.is(t::NAME) || sub.is(t::FIELD) || sub.is(t::INDEX) {
                    return result;
                }
                vec![1]
            }
            t::UNSAFE | t::LOAD | t::BITWISE_NOT | t::LOGICAL_NOT => vec![1],
            t::ASSIGN | t::CAST => vec![2],
            t::CONDITIONAL => {
                let condition = self.fold(child(expr, 1));
                if let Some(value) = condition.int(1).filter(|_| condition.is(t::INTEGER)) {
                    return self.fold(child(expr, if value != 0 { 2 } else { 3 }));
                }
                result.args[1] = condition.into();
                vec![2, 3]
            }
            t::CALL => {
                let target = self.fold(child(expr, 1));
                result.args[1] = target.into();
                if child(expr, 1).is(t::NAME) && child(expr, 1).text(1) == Some("__bankof") {
                    Vec::new()
                } else {
                    (2..expr.args.len()).collect()
                }
            }
            t::SLICE | t::INDEX => vec![1, 2],
            t::FIELD => vec![1],
            t::ADD
            | t::SUBTRACT
            | t::MULTIPLY
            | t::DIVIDE
            | t::MODULUS
            | t::BITWISE_AND
            | t::BITWISE_OR
            | t::BITWISE_XOR
            | t::SHIFT_LEFT
            | t::SHIFT_RIGHT
            | t::EQUAL
            | t::NOT_EQUAL
            | t::LESS_THAN
            | t::LESS_THAN_OR_EQUAL
            | t::GREATER_THAN
            | t::GREATER_THAN_OR_EQUAL
            | t::LOGICAL_AND
            | t::LOGICAL_OR => {
                let left = self.fold(child(expr, 1));
                if left.is(t::INTEGER) {
                    let value = left.int(1).unwrap();
                    if expr.is(t::LOGICAL_AND) && value == 0 {
                        return Expr::new(t::INTEGER, vec![0.into()])
                            .with_source(expr.source.clone());
                    }
                    if expr.is(t::LOGICAL_OR) && value != 0 {
                        return Expr::new(t::INTEGER, vec![1.into()])
                            .with_source(expr.source.clone());
                    }
                }
                result.args[1] = left.into();
                vec![2]
            }
            _ => return result,
        };
        for index in indexes {
            result.args[index] = self.fold(child(expr, index)).into();
        }
        if !matches!(
            expr.tag(),
            Some(t::ADDRESS_OF | t::UNSAFE | t::ASSIGN | t::LOAD | t::SLICE | t::INDEX | t::FIELD)
        ) {
            if let Some(value) = self.try_constant(&result) {
                let integer = Expr::new(t::INTEGER, vec![value.value.into()])
                    .with_source(expr.source.clone());
                return if expr.is(t::CAST) {
                    Expr::new(t::CAST, vec![expr.args[1].clone(), integer.into()])
                        .with_source(expr.source.clone())
                } else {
                    integer
                };
            }
        }
        result
    }
}
