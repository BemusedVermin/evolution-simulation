//! Pure evaluator for [`CompiledExpr`] over a per-channel value map.
//!
//! Evaluation is a fold over Q32.32 saturating arithmetic; see
//! [`eval_expression`] for the determinism contract.

use std::collections::BTreeMap;

use beast_core::Q3232;

use super::ast::{CompiledExpr, ExprNode};

/// Evaluate a parsed expression against per-channel global values.
///
/// Returns [`Q3232::ZERO`] for any channel id not present in
/// `channel_values` — this matches the "dormant channels propagate zero"
/// rule from §6.2. Addition and multiplication use [`Q3232`] saturating
/// arithmetic, so overflow clamps to [`Q3232::MAX`] / [`Q3232::MIN`] rather
/// than panicking or wrapping.
///
/// This function is pure: the output is entirely determined by
/// `(expr, channel_values)`, satisfying the determinism invariant
/// (INVARIANTS §1).
#[must_use]
pub fn eval_expression(expr: &CompiledExpr, channel_values: &BTreeMap<String, Q3232>) -> Q3232 {
    eval_node(expr.node(), channel_values)
}

fn eval_node(node: &ExprNode, channel_values: &BTreeMap<String, Q3232>) -> Q3232 {
    match node {
        ExprNode::Literal(v) => *v,
        ExprNode::ChannelRef(id) => channel_values.get(id).copied().unwrap_or(Q3232::ZERO),
        ExprNode::Add(lhs, rhs) => {
            eval_node(lhs, channel_values).saturating_add(eval_node(rhs, channel_values))
        }
        ExprNode::Mul(lhs, rhs) => {
            eval_node(lhs, channel_values).saturating_mul(eval_node(rhs, channel_values))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parameter_map::parse_expression;
    use crate::parameter_map::test_support::{channels, registry_with};
    use proptest::prelude::*;

    #[test]
    fn evaluates_literal() {
        let vals = BTreeMap::new();
        let expr = CompiledExpr::from_node(ExprNode::Literal(Q3232::from_num(3_i32)));
        let out = eval_expression(&expr, &vals);
        assert_eq!(out, Q3232::from_num(3_i32));
    }

    #[test]
    fn evaluates_channel_ref_present() {
        let vals = channels(&[("a", Q3232::from_num(5_i32))]);
        let expr = CompiledExpr::from_node(ExprNode::ChannelRef("a".into()));
        let out = eval_expression(&expr, &vals);
        assert_eq!(out, Q3232::from_num(5_i32));
    }

    #[test]
    fn evaluates_channel_ref_missing_as_zero() {
        // The "dormant channels propagate zero" rule: an AST that refers
        // to a channel not present in the runtime value map reads as ZERO.
        let vals = BTreeMap::new();
        let expr = CompiledExpr::from_node(ExprNode::ChannelRef("missing".into()));
        let out = eval_expression(&expr, &vals);
        assert_eq!(out, Q3232::ZERO);
    }

    #[test]
    fn roundtrip_parse_then_evaluate() {
        let reg = registry_with(&["a", "b"]);
        let expr = parse_expression("ch[a] * 8 + ch[b]", &reg).unwrap();
        let vals = channels(&[
            ("a", Q3232::from_num(0.5_f64)),
            ("b", Q3232::from_num(2_i32)),
        ]);
        // 0.5 * 8 + 2 = 6
        let out = eval_expression(&expr, &vals);
        assert_eq!(out, Q3232::from_num(6_i32));
    }

    #[test]
    fn roundtrip_same_input_yields_same_output_twice() {
        let reg = registry_with(&["a", "b"]);
        let expr = parse_expression("ch[a] * 8 + ch[b]", &reg).unwrap();
        let vals = channels(&[
            ("a", Q3232::from_num(0.25_f64)),
            ("b", Q3232::from_num(1_i32)),
        ]);
        let first = eval_expression(&expr, &vals);
        let second = eval_expression(&expr, &vals);
        assert_eq!(first, second);
    }

    #[test]
    fn saturating_addition_clamps() {
        let vals = BTreeMap::new();
        // Driving the evaluator with a hand-built AST is fine from inside
        // the module — `ExprNode` is `pub(crate)` and exposed here via
        // `super::*`. External crates cannot do this: they must go through
        // [`parse_expression`], which is the whole point of [`CompiledExpr`].
        let expr = CompiledExpr::from_node(ExprNode::Add(
            Box::new(ExprNode::Literal(Q3232::MAX)),
            Box::new(ExprNode::Literal(Q3232::ONE)),
        ));
        assert_eq!(eval_expression(&expr, &vals), Q3232::MAX);
    }

    // ------- proptest: evaluator purity -----------------------------------

    /// Sample a small, balanced AST. We intentionally keep the channel
    /// alphabet small so the "missing channel → zero" path is exercised.
    ///
    /// This proptest drives the evaluator's internal [`ExprNode`] type
    /// directly (not [`CompiledExpr`]) because the point of the test is
    /// to prove the evaluator is a deterministic fold — it should not
    /// matter how the AST was produced, only that the same input always
    /// yields the same output.
    fn arb_node() -> impl Strategy<Value = ExprNode> {
        let leaf = prop_oneof![
            (-100_000_i64..=100_000_i64).prop_map(|n| ExprNode::Literal(Q3232::from_num(n))),
            prop::sample::select(vec!["a", "b", "c", "d", "missing"])
                .prop_map(|s| ExprNode::ChannelRef(s.to_string())),
        ];
        leaf.prop_recursive(4, 16, 2, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(l, r)| ExprNode::Add(Box::new(l), Box::new(r))),
                (inner.clone(), inner).prop_map(|(l, r)| ExprNode::Mul(Box::new(l), Box::new(r))),
            ]
        })
    }

    proptest! {
        /// `eval_expression` is a pure function of `(expr, channel_values)`:
        /// calling it twice on the same inputs always yields the same output.
        /// Required by INVARIANTS §1 (determinism).
        #[test]
        fn eval_is_pure_and_deterministic(
            node in arb_node(),
            bits_a in any::<i64>(),
            bits_b in any::<i64>(),
            bits_c in any::<i64>(),
            bits_d in any::<i64>(),
        ) {
            let expr = CompiledExpr::from_node(node);
            let vals = channels(&[
                ("a", Q3232::from_bits(bits_a)),
                ("b", Q3232::from_bits(bits_b)),
                ("c", Q3232::from_bits(bits_c)),
                ("d", Q3232::from_bits(bits_d)),
            ]);
            let first = eval_expression(&expr, &vals);
            let second = eval_expression(&expr, &vals);
            prop_assert_eq!(first, second);
        }
    }
}
