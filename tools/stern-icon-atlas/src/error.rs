//! Structured deterministic errors.

use std::{fmt, io, path::Path};

/// Result returned by icon-atlas operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Stable category for an ingestion failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// Source bytes could not be read.
    Io,
    /// The source archive or one of its entries is malformed.
    Archive,
    /// Pinned provenance did not match.
    Provenance,
    /// Official catalog metadata is malformed or inconsistent.
    Catalog,
    /// Weight assets are missing, extra, or inconsistent.
    Discovery,
    /// An SVG document or path is invalid or unsupported.
    Svg,
    /// Two public names normalize to the same Rust identifier.
    NameCollision,
    /// Two icon definitions received the same stable identifier.
    IdCollision,
}

/// Deterministic ingestion error with stable context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    /// Error category.
    pub kind: ErrorKind,
    /// Source entry, icon, or field involved in the failure.
    pub context: String,
    /// Human-readable explanation.
    pub message: String,
}

impl Error {
    pub(crate) fn new(
        kind: ErrorKind,
        context: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            context: context.into(),
            message: message.into(),
        }
    }

    pub(crate) fn io(path: &Path, error: &io::Error) -> Self {
        Self::new(ErrorKind::Io, path.display().to_string(), error.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} [{}]: {}",
            self.kind, self.context, self.message
        )
    }
}

impl std::error::Error for Error {}
