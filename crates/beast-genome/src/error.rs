//! Error types for genome construction and validation.

use thiserror::Error;

/// Errors produced while constructing or validating genome types.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum GenomeError {
    /// A `Q3232` field was outside its declared [0, 1] canonical range.
    #[error("{field} must be in [0, 1]; got {value}")]
    OutOfUnitRange {
        /// Field name (e.g. `"magnitude"`, `"radius"`).
        field: &'static str,
        /// Debug-formatted value from the offending `Q3232`.
        value: String,
    },

    /// A modifier pointed at a gene index that doesn't exist.
    #[error("modifier target_gene_index {index} is out of bounds (genome has {len} genes)")]
    ModifierIndexOutOfBounds {
        /// The offending index.
        index: u32,
        /// Current genome length.
        len: usize,
    },

    /// A modifier's strength was outside `[-1, 1]`.
    #[error("modifier strength must be in [-1, 1]; got {value}")]
    ModifierStrengthOutOfRange {
        /// Debug-formatted strength.
        value: String,
    },

    /// A modifier targets its own gene — self-loops are forbidden.
    #[error(
        "modifier target_gene_index equals source gene index ({index}); self-loops are forbidden"
    )]
    ModifierSelfLoop {
        /// Offending index.
        index: u32,
    },

    /// A channel vector had a different length than the registry declared at
    /// construction time.
    #[error("effect vector has {got} channels, expected {expected}")]
    ChannelCountMismatch {
        /// Expected channel count (from registry).
        expected: usize,
        /// Actual channel count on the gene.
        got: usize,
    },

    /// Two genes shared the same `LineageTag`.
    #[error("duplicate lineage tag {tag} within genome")]
    DuplicateLineageTag {
        /// Offending tag, as `u64`.
        tag: u64,
    },
}

/// Convenience `Result` alias for genome-crate errors.
pub type Result<T> = core::result::Result<T, GenomeError>;
