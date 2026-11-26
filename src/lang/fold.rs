//! Docs: dev:validator-typing, llm:validator
//!
//! Shared full-precision constant folding (Phase 18 task 3L).
//!
//! A *constant expression* is a tree of integer literals, arithmetic and
//! bitwise operators, and unary minus - no paths, no booleans, no comparisons.
//! Such a tree is semantically an INTEGER LITERAL: it folds to an exact signed
//! `i128` value with NO silent truncation, wrap, or width assumption. Bitwise
//! operators on negative constants use the native `i128` two's-complement bit
//! pattern (the unsigned two's-complement representation), which is exactly the
//! required rule-1 interpretation (`(-5) & 15` folds to the positive constant
//! 11). Overflow of a signed intermediate is an error, never a silent wrap.
//!
//! The IR keeps expressions UNFOLDED (printer + JSON round-trip untouched); this
//! stage is the single shared translation that validation, mc-ref, and every
//! codegen backend call instead of re-deriving widths from `IntLit`.

use crate::lang::ir::{BinOp, Expr, UnaryOp};

/// True when `e` is a constant expression: integer literals + arithmetic /
/// bitwise binary operators + unary minus / bit-not only. Any `Path`, `Bool`,
/// logical operator, comparison, or boolean negation breaks constness.
pub fn is_const_expr(e: &Expr) -> bool {
    match e {
        Expr::Int { .. } => true,
        Expr::Unary { op, expr } => {
            matches!(op, UnaryOp::Neg | UnaryOp::BitNot) && is_const_expr(expr)
        }
        Expr::Binary { op, left, right } => {
            matches!(
                op,
                BinOp::Plus
                    | BinOp::Minus
                    | BinOp::Star
                    | BinOp::ShiftLeft
                    | BinOp::ShiftRight
                    | BinOp::BitAnd
                    | BinOp::BitOr
                    | BinOp::BitXor
            ) && is_const_expr(left)
                && is_const_expr(right)
        }
        _ => false,
    }
}

/// Fold a constant expression to its exact signed value. Overflow of any
/// signed intermediate is an error (never a silent wrap). Bitwise and shift
/// operators use the native `i128` two's-complement bit pattern. The caller
/// must have checked [`is_const_expr`].
pub fn fold_const(e: &Expr) -> Result<i128, String> {
    match e {
        Expr::Int { value } => Ok(*value as i128),
        Expr::Unary { op, expr } => {
            let v = fold_const(expr)?;
            match op {
                UnaryOp::Neg => v
                    .checked_neg()
                    .ok_or_else(|| "constant expression overflows".to_string()),
                UnaryOp::BitNot => Ok(!v),
                UnaryOp::Not => unreachable!("is_const_expr excludes boolean negation"),
            }
        }
        Expr::Binary { op, left, right } => {
            let a = fold_const(left)?;
            let b = fold_const(right)?;
            match op {
                BinOp::Plus => a
                    .checked_add(b)
                    .ok_or_else(|| "constant expression overflows".to_string()),
                BinOp::Minus => a
                    .checked_sub(b)
                    .ok_or_else(|| "constant expression overflows".to_string()),
                BinOp::Star => a
                    .checked_mul(b)
                    .ok_or_else(|| "constant expression overflows".to_string()),
                BinOp::ShiftLeft => {
                    if !(0..128).contains(&b) {
                        Ok(0)
                    } else {
                        a.checked_shl(b as u32)
                            .ok_or_else(|| "constant expression overflows".to_string())
                    }
                }
                BinOp::ShiftRight => {
                    if !(0..128).contains(&b) {
                        Ok(if a < 0 { -1 } else { 0 })
                    } else {
                        Ok(a >> b as u32)
                    }
                }
                BinOp::BitAnd => Ok(a & b),
                BinOp::BitOr => Ok(a | b),
                BinOp::BitXor => Ok(a ^ b),
                _ => unreachable!("is_const_expr excludes non-constant operators"),
            }
        }
        _ => unreachable!("fold_const called on a non-constant expression"),
    }
}

/// Replace every maximal constant subexpression by its folded literal, to a
/// fixpoint. Only folds when the result is non-negative and fits `u64` (the IR
/// `Expr::Int` representation); negative or oversized results stay unfolded and
/// are converted at their width boundary via [`fold_const`] instead. This is an
/// optional normalization for consumers that render folded constants as bare
/// literals; the IR itself keeps expressions unfolded.
pub fn fold_tree(e: &Expr) -> Expr {
    match e {
        Expr::Binary { op, left, right } => {
            let l = fold_tree(left);
            let r = fold_tree(right);
            if is_const_expr(&Expr::Binary {
                op: *op,
                left: Box::new(l.clone()),
                right: Box::new(r.clone()),
            }) {
                if let Ok(v) = fold_const(&Expr::Binary {
                    op: *op,
                    left: Box::new(l.clone()),
                    right: Box::new(r.clone()),
                }) {
                    if (0..=u64::MAX as i128).contains(&v) {
                        return Expr::Int { value: v as u64 };
                    }
                }
            }
            Expr::Binary {
                op: *op,
                left: Box::new(l),
                right: Box::new(r),
            }
        }
        Expr::Unary { op, expr } => {
            let inner = fold_tree(expr);
            let cand = Expr::Unary {
                op: *op,
                expr: Box::new(inner.clone()),
            };
            if is_const_expr(&cand) {
                if let Ok(v) = fold_const(&cand) {
                    if (0..=u64::MAX as i128).contains(&v) {
                        return Expr::Int { value: v as u64 };
                    }
                }
            }
            Expr::Unary {
                op: *op,
                expr: Box::new(inner),
            }
        }
        other => other.clone(),
    }
}

/// Rule 4 (task 3L): convert a folded constant `c` into a `bit<N>` context.
/// Positive `c < 2^N` is OK with no diagnostic; positive `c >= 2^N` is an error
/// naming the value, width, and the cast escape. Negative `c` whose two's-
/// complement form fits (`abs(c) <= 2^(N-1)`) is a WARNING (value = two's
/// complement); negative `c` that does not fit is an error. The explicit
/// `bit<N>(...)` cast is the only way to suppress these - it never reaches here.
pub(crate) fn const_fit(c: i128, n: u32, warns: &mut Vec<String>) -> Result<(), String> {
    if c >= 0 {
        let max = if n >= 127 {
            i128::MAX
        } else {
            (1i128 << n) - 1
        };
        if c > max {
            Err(format!(
                "constant {} does not fit in bit<{}>; use an explicit cast, e.g. bit<{}>(...)",
                c, n, n
            ))
        } else {
            Ok(())
        }
    } else {
        let abs = -c;
        let half = if n >= 127 {
            i128::MAX / 2
        } else {
            1i128 << (n - 1)
        };
        if abs <= half {
            warns.push(format!(
                "implicit conversion of negative constant to unsigned bit-vector: {} becomes {} (two's complement in bit<{}>)",
                c,
                c & ((1i128 << n) - 1),
                n
            ));
            Ok(())
        } else {
            Err(format!(
                "constant {} does not fit in bit<{}> (its two's-complement form needs {} bits); use an explicit cast, e.g. bit<{}>(...)",
                c,
                n,
                n + 1,
                n
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::ir::{BinOp, Expr, UnaryOp};

    fn lit(v: u64) -> Expr {
        Expr::Int { value: v }
    }
    fn bin(op: BinOp, l: Expr, r: Expr) -> Expr {
        Expr::Binary {
            op,
            left: Box::new(l),
            right: Box::new(r),
        }
    }
    fn neg(e: Expr) -> Expr {
        Expr::Unary {
            op: UnaryOp::Neg,
            expr: Box::new(e),
        }
    }

    #[test]
    fn full_precision_pins() {
        // 200 * 200 = 40000, NOT 16 (the old bit<8> cap).
        assert_eq!(
            fold_const(&bin(BinOp::Star, lit(200), lit(200))).unwrap(),
            40000
        );
        // Unary minus is signed.
        assert_eq!(fold_const(&neg(lit(5))).unwrap(), -5);
    }

    #[test]
    fn bitwise_two_complement_pins() {
        // (-5) & 15 folds to the POSITIVE constant 11.
        assert_eq!(
            fold_const(&bin(BinOp::BitAnd, neg(lit(5)), lit(15))).unwrap(),
            11
        );
        // 2 + (-1) & 1 : & binds tighter in the AST we build, (-1)&1 = 1, 2+1 = 3.
        assert_eq!(
            fold_const(&bin(
                BinOp::Plus,
                lit(2),
                bin(BinOp::BitAnd, neg(lit(1)), lit(1))
            ))
            .unwrap(),
            3
        );
        // 2 + (-4) & 3 : (-4)&3 = 0, 2+0 = 2.
        assert_eq!(
            fold_const(&bin(
                BinOp::Plus,
                lit(2),
                bin(BinOp::BitAnd, neg(lit(4)), lit(3))
            ))
            .unwrap(),
            2
        );
    }

    #[test]
    fn overflow_is_an_error() {
        // u64::MAX * u64::MAX ~= 2^128 exceeds i128.
        let big = bin(BinOp::Star, lit(u64::MAX), lit(u64::MAX));
        assert!(fold_const(&big).is_err(), "i128 overflow must be detected");
        // 2^63 * 2^63 = 2^126 still fits - no false positive.
        let ok = bin(
            BinOp::Star,
            lit(9223372036854775808),
            lit(9223372036854775808),
        );
        assert!(fold_const(&ok).is_ok(), "2^126 fits in i128");
    }

    #[test]
    fn non_const_detection() {
        // A Path operand is not const.
        let path = Expr::Path {
            path: vec!["x".into()],
        };
        assert!(!is_const_expr(&bin(BinOp::Plus, path, lit(1))));
        assert!(is_const_expr(&lit(5)));
        assert!(!is_const_expr(&Expr::Bool { value: true }));
        // A comparison is not a constant expression.
        assert!(!is_const_expr(&bin(BinOp::Eq, lit(1), lit(1))));
    }

    #[test]
    fn fold_tree_folds_nonnegative() {
        // (300 & 255) = 44 folds to a bare literal.
        let e = bin(BinOp::BitAnd, lit(300), lit(255));
        assert!(matches!(fold_tree(&e), Expr::Int { value: 44 }));
        // A negative result stays unfolded (not representable in Expr::Int).
        let n = neg(lit(5));
        assert!(!matches!(fold_tree(&n), Expr::Int { .. }));
    }
}
