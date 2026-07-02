use kinetik_ui_core::{AccessibilitySnapshot, FrameOutput, SemanticTreeError, WidgetId};
/// Accessibility update ready for a winit-hosted platform adapter.
///
/// This type is intentionally free of OS accessibility APIs. Application shells
/// can translate the snapshot into Windows, macOS, Linux, or test adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct WinitAccessibilityUpdate {
    /// Validated accessibility snapshot exported from the core frame.
    pub snapshot: AccessibilitySnapshot,
}

impl WinitAccessibilityUpdate {
    /// Translates core frame output into winit-facing accessibility data.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticTreeError`] when the frame's semantic tree is
    /// structurally invalid.
    pub fn from_frame_output(
        output: &FrameOutput,
        focused: Option<WidgetId>,
    ) -> Result<Self, SemanticTreeError> {
        output
            .accessibility_snapshot(focused)
            .map(|snapshot| Self { snapshot })
    }
}
