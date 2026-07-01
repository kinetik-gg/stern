//! Overlay, menu, popover, and command palette models.

mod command_palette;
mod dropdown;
mod menu;
mod modal;
mod model;
mod placement;
mod semantics;
mod stack;

pub use command_palette::{CommandPalette, CommandPaletteEntry, CommandPaletteOverlay};
pub use dropdown::{
    DropdownCloseReason, DropdownCloseResult, DropdownHighlightMove, DropdownItem, DropdownItemId,
    DropdownModel, DropdownOverlay, DropdownTriggerPresentation, DropdownVisibleRange,
    dropdown_visible_range,
};
pub use menu::{Menu, MenuItem, MenuOverlay};
pub use modal::{
    ModalAction, ModalActionRole, ModalCloseReason, ModalCloseRequest, ModalDialog,
    ModalDialogBody, ModalDialogOverlay, ModalFocusContainment,
};
pub use model::{OverlayDismissal, OverlayEntry, OverlayId, OverlayKind};
pub use placement::{PopoverPlacement, PopoverRequest, place_popover};
pub use semantics::overlay_semantics;
pub use stack::OverlayStack;

#[cfg(test)]
mod tests;
