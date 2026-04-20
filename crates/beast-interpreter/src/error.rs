//! Error type for the interpreter crate.
//!
//! Errors surface when callers feed the interpreter something the registries
//! cannot satisfy — an unknown channel symbol in a parameter expression, an
//! unknown primitive id on an [`crate::EmitSpec`], or a malformed manifest at
//! load time. Evaluation-time paths (hook resolution, emission) never fail —
//! they return empty sets rather than errors, consistent with the design
//! doc's "dormant channels propagate zero" semantics.

use thiserror::Error;

/// Errors produced by the interpreter.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum InterpreterError {
    /// A parameter expression referenced a channel symbol that is not
    /// registered.
    #[error("unknown channel symbol `{symbol}` in expression")]
    UnknownChannelSymbol {
        /// The offending symbol as it appeared in the source expression.
        symbol: String,
    },

    /// An [`crate::EmitSpec`] references a primitive id that is not in the
    /// primitive registry.
    #[error("unknown primitive id `{primitive_id}` in emit spec")]
    UnknownPrimitive {
        /// The missing primitive id.
        primitive_id: String,
    },

    /// A parameter expression could not be parsed.
    #[error("invalid parameter expression: {message}")]
    ParseError {
        /// Human-readable description of what failed to parse.
        message: String,
    },
}

/// Crate-specific `Result` alias.
pub type Result<T> = core::result::Result<T, InterpreterError>;
