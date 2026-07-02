//! Platform-independent accessibility semantics.

mod adapter;
mod focus;
mod model;
mod snapshot;
#[cfg(test)]
mod tests;
mod tree;

pub use adapter::AccessibilityAdapter;
pub use focus::FocusTraversal;
pub use model::{
    SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticState, SemanticValue,
};
pub use snapshot::{AccessibilityNode, AccessibilitySnapshot};
pub use tree::{SemanticTree, SemanticTreeError};
