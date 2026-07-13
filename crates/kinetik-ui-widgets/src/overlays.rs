//! Overlay, menu, popover, and command palette models.

mod command_palette;
mod dropdown;
mod menu;
mod modal;
mod model;
mod navigation;
mod placement;
mod scene;
mod semantics;
mod stack;

pub use command_palette::{CommandPalette, CommandPaletteEntry, CommandPaletteOverlay};
pub use dropdown::{
    DropdownCloseReason, DropdownCloseResult, DropdownHighlightMove, DropdownItem, DropdownItemId,
    DropdownModel, DropdownNavigationIntent, DropdownOverlay, DropdownTriggerPresentation,
    DropdownVisibleRange, dropdown_visible_range,
};
pub use menu::{Menu, MenuItem, MenuNavigationIntent, MenuOverlay, MenuSubmenuOpenIntent};
pub use modal::{
    ModalAction, ModalActionRole, ModalCloseReason, ModalCloseRequest, ModalDialog,
    ModalDialogBody, ModalDialogOverlay, ModalFocusContainment,
};
pub use model::{OverlayDismissal, OverlayEntry, OverlayId, OverlayKind};
pub use navigation::{OverlayNavigationInput, TypeaheadBuffer};
pub use placement::{PopoverPlacement, PopoverRequest, place_popover};
pub use scene::{
    OverlayScene, OverlaySceneDismissReason, OverlaySceneDismissRequest,
    OverlaySceneDropdownSelection, OverlaySceneIntent, OverlaySceneMetrics, OverlaySceneOutput,
    OverlaySceneSurface,
};
pub(crate) use scene::{OverlaySceneRow, OverlaySceneRowKind};
pub use semantics::overlay_semantics;
pub use stack::OverlayStack;

#[cfg(test)]
mod tests;
