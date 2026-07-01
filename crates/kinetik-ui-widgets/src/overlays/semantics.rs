use kinetik_ui_core::{SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, WidgetId};

use super::{OverlayDismissal, OverlayEntry, OverlayKind};
/// Builds a semantic node for an overlay surface.
#[must_use]
pub fn overlay_semantics(entry: &OverlayEntry, label: impl Into<String>) -> SemanticNode {
    let role = match entry.kind {
        OverlayKind::Menu | OverlayKind::ContextMenu | OverlayKind::Dropdown => SemanticRole::Menu,
        OverlayKind::CommandPalette => SemanticRole::CommandPalette,
        OverlayKind::Popover => SemanticRole::Custom("popover".to_owned()),
        OverlayKind::Tooltip => SemanticRole::Custom("tooltip".to_owned()),
        OverlayKind::Modal => SemanticRole::Custom("modal".to_owned()),
        OverlayKind::DragPreview => SemanticRole::Custom("drag-preview".to_owned()),
    };
    let mut node =
        SemanticNode::new(WidgetId::from_raw(entry.id.raw()), role, entry.rect).with_label(label);
    if entry.receives_focus() {
        node = node.focusable(true);
    }
    if entry.dismissal != OverlayDismissal::Manual {
        node = node.with_action(SemanticAction::new(SemanticActionKind::Dismiss, "Dismiss"));
    }
    node
}
