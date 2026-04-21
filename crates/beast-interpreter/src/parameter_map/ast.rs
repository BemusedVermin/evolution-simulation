//! Expression AST for the S4.4 parameter-mapping language.
//!
//! The AST is split from the parser (`super::parser`) so the evaluator and
//! analysis passes can consume it without depending on the parser's
//! recursive-descent machinery.
//!
//! ## Type-system enforcement of "channel symbols resolved at parse time"
//!
//! Channel symbols **must** be resolved against a
//! [`beast_channels::ChannelRegistry`] at parse time (sprint plan Q4;
//! §6.2). To enforce this in the type system — rather than by convention —
//! the raw AST [`ExprNode`] has **crate-private variants**: downstream
//! crates cannot construct an `ExprNode::ChannelRef(unresolved)`.
//!
//! The only path by which a downstream crate obtains an expression value is
//! [`super::parse_expression`], which returns a [`CompiledExpr`]. Every
//! [`CompiledExpr`] is therefore, by construction, a fully-resolved
//! expression whose channel references were checked against the registry.
//!
//! [`Expr`] is a backward-compatibility alias for [`CompiledExpr`] used by
//! downstream crates that stored expressions in `Vec<(String, Expr)>`-shaped
//! fields prior to the S4.6 split (issue #76).

use beast_core::Q3232;

/// Raw expression AST — **crate-private** by design.
///
/// The variants are `pub(crate)` so that only the parser (which validates
/// channel symbols against the registry) and the evaluator / analyser
/// (which consume parsed values) can construct or match them. External
/// crates interact with expressions exclusively through [`CompiledExpr`].
#[derive(Debug, Clone)]
pub(crate) enum ExprNode {
    /// A literal Q32.32 value.
    Literal(Q3232),
    /// Reference to a channel value by resolved id.
    ChannelRef(String),
    /// Binary addition.
    Add(Box<ExprNode>, Box<ExprNode>),
    /// Binary multiplication.
    Mul(Box<ExprNode>, Box<ExprNode>),
}

/// A parsed-and-resolved parameter-mapping expression.
///
/// "Resolved" means every `ch[<symbol>]` in the source was looked up in a
/// [`beast_channels::ChannelRegistry`] at parse time and rejected if
/// unknown. Because the inner [`ExprNode`] has crate-private variants, a
/// `CompiledExpr` can **only** be produced by [`super::parse_expression`]
/// — downstream crates cannot fabricate an unresolved expression and then
/// cast it into a `CompiledExpr`.
///
/// Evaluation ([`super::eval_expression`]) and channel-ref collection
/// ([`super::collect_channel_refs`]) both take `&CompiledExpr`, so the
/// resolution guarantee propagates through the entire pipeline.
#[derive(Debug, Clone)]
pub struct CompiledExpr {
    pub(crate) node: ExprNode,
}

impl CompiledExpr {
    /// Construct a `CompiledExpr` from a crate-private AST node.
    ///
    /// This is `pub(crate)` so only the parser can mint a `CompiledExpr`.
    pub(crate) fn from_node(node: ExprNode) -> Self {
        Self { node }
    }

    /// Borrow the inner AST node for internal consumers (evaluator,
    /// analyser). Not exposed publicly — downstream crates should not rely
    /// on the AST shape.
    pub(crate) fn node(&self) -> &ExprNode {
        &self.node
    }
}

/// Backward-compatibility alias for [`CompiledExpr`].
///
/// Pre-S4.6 (issue #76), the public AST type was `Expr`. Downstream code
/// stores expressions in typed fields like
/// `parameter_mapping: Vec<(String, Expr)>`. Keeping `Expr` as a type
/// alias preserves that call-site ergonomy while the underlying type is
/// now the newtype that enforces resolution.
pub type Expr = CompiledExpr;
