use kinetik_ui_core::Rect;

/// Stable overlay identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OverlayId(u64);

impl OverlayId {
    /// Creates an overlay ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Overlay kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    /// Popover surface.
    Popover,
    /// Dropdown surface anchored to a control.
    Dropdown,
    /// Context menu opened from a contextual target.
    ContextMenu,
    /// Menu surface.
    Menu,
    /// Command palette surface.
    CommandPalette,
    /// Tooltip surface.
    Tooltip,
    /// Modal overlay that blocks interaction with lower layers.
    Modal,
    /// Drag preview surface.
    DragPreview,
}

/// Dismissal behavior for an overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayDismissal {
    /// Overlay remains open until explicitly closed.
    Manual,
    /// Overlay closes when the pointer activates outside its bounds.
    OutsideClick,
    /// Overlay closes when Escape is pressed.
    Escape,
    /// Overlay closes when either outside activation or Escape occurs.
    OutsideClickOrEscape,
}

impl OverlayDismissal {
    pub(crate) fn closes_on_outside_click(self) -> bool {
        matches!(self, Self::OutsideClick | Self::OutsideClickOrEscape)
    }

    pub(crate) fn closes_on_escape(self) -> bool {
        matches!(self, Self::Escape | Self::OutsideClickOrEscape)
    }
}

/// Overlay entry in top-to-bottom ordering.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayEntry {
    /// Overlay identity.
    pub id: OverlayId,
    /// Parent overlay for nested menu/popover behavior.
    pub parent: Option<OverlayId>,
    /// Overlay kind.
    pub kind: OverlayKind,
    /// Overlay bounds.
    pub rect: Rect,
    /// Whether this overlay captures interaction before lower overlays.
    pub modal: bool,
    /// Dismissal behavior.
    pub dismissal: OverlayDismissal,
}

impl OverlayEntry {
    /// Creates a manual non-modal overlay entry.
    #[must_use]
    pub const fn new(id: OverlayId, kind: OverlayKind, rect: Rect) -> Self {
        Self {
            id,
            parent: None,
            kind,
            rect,
            modal: false,
            dismissal: OverlayDismissal::Manual,
        }
    }

    /// Returns this entry with a parent overlay.
    #[must_use]
    pub const fn with_parent(mut self, parent: OverlayId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Returns this entry with modality set.
    #[must_use]
    pub const fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    /// Returns this entry with dismissal behavior set.
    #[must_use]
    pub const fn dismiss_on(mut self, dismissal: OverlayDismissal) -> Self {
        self.dismissal = dismissal;
        self
    }

    pub(crate) fn captures_lower_layers(&self) -> bool {
        self.modal || self.kind == OverlayKind::Modal
    }

    pub(crate) fn receives_focus(&self) -> bool {
        self.captures_lower_layers()
            || matches!(
                self.kind,
                OverlayKind::Menu
                    | OverlayKind::Dropdown
                    | OverlayKind::ContextMenu
                    | OverlayKind::CommandPalette
            )
    }
}
