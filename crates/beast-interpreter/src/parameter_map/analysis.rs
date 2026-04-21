//! Static analysis passes over a parsed [`CompiledExpr`].
//!
//! Currently exposes [`collect_channel_refs`]; future passes (dead-channel
//! detection, constant folding) live here without bloating the evaluator.

use std::collections::BTreeSet;

use super::ast::{CompiledExpr, ExprNode};

/// Return every channel id referenced by `expr`, sorted and deduplicated.
///
/// Used by the emission path to compose a hook's
/// [`beast_primitives::PrimitiveEffect::source_channels`] without having
/// to re-walk the raw manifest source string.
#[must_use]
pub fn collect_channel_refs(expr: &CompiledExpr) -> Vec<String> {
    let mut out = BTreeSet::new();
    walk(expr.node(), &mut out);
    out.into_iter().collect()
}

fn walk(node: &ExprNode, out: &mut BTreeSet<String>) {
    match node {
        ExprNode::Literal(_) => {}
        ExprNode::ChannelRef(id) => {
            out.insert(id.clone());
        }
        ExprNode::Add(l, r) | ExprNode::Mul(l, r) => {
            walk(l, out);
            walk(r, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parameter_map::parse_expression;
    use crate::parameter_map::test_support::registry_with;

    #[test]
    fn collect_channel_refs_returns_sorted_unique_ids() {
        let reg = registry_with(&["alpha", "beta"]);
        let expr = parse_expression("ch[beta] + ch[alpha] * 2 + ch[beta]", &reg).unwrap();
        let refs = collect_channel_refs(&expr);
        assert_eq!(refs, vec!["alpha".to_string(), "beta".to_string()]);
    }
}
