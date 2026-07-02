use super::{AccessibilitySnapshot, SemanticActionKind};
use crate::WidgetId;

/// Boundary implemented by platform-specific accessibility adapters.
pub trait AccessibilityAdapter {
    /// Adapter error type.
    type Error;

    /// Synchronizes the platform accessibility tree with the current snapshot.
    ///
    /// # Errors
    ///
    /// Returns an adapter-specific error when platform synchronization fails.
    fn synchronize(&mut self, snapshot: &AccessibilitySnapshot) -> Result<(), Self::Error>;

    /// Notifies the platform that focus moved.
    ///
    /// # Errors
    ///
    /// Returns an adapter-specific error when the platform focus update fails.
    fn focus(&mut self, node: WidgetId) -> Result<(), Self::Error>;

    /// Requests a semantic action on a node.
    ///
    /// # Errors
    ///
    /// Returns an adapter-specific error when the platform cannot perform the action.
    fn perform_action(
        &mut self,
        node: WidgetId,
        action: &SemanticActionKind,
    ) -> Result<(), Self::Error>;
}
