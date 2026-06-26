//! Data-only taxonomy metadata for Kinetik UI widget components.

/// Kinetik-owned component category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComponentCategory {
    /// Static display and decoration components.
    Display,
    /// Clickable, selectable, or adjustable controls.
    Control,
    /// Non-text input controls.
    Input,
    /// Text editing and text-query controls.
    TextEditing,
    /// Collection, virtualization, and structured data components.
    Collection,
    /// Docking, frame, and panel workspace components.
    Docking,
    /// Menus, popovers, command palettes, and other overlay surfaces.
    Overlay,
    /// Media, image, video, and editor viewport surfaces.
    Viewport,
    /// Property editing and inspector patterns.
    Inspector,
    /// System-level editor chrome and status patterns.
    System,
}

impl ComponentCategory {
    /// Returns a stable display name for the category.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::Control => "Control",
            Self::Input => "Input",
            Self::TextEditing => "TextEditing",
            Self::Collection => "Collection",
            Self::Docking => "Docking",
            Self::Overlay => "Overlay",
            Self::Viewport => "Viewport",
            Self::Inspector => "Inspector",
            Self::System => "System",
        }
    }
}

/// Honest implementation status for a component or editor pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComponentConformanceStatus {
    /// Public widget behavior exists for common usage.
    Implemented,
    /// Public models, helpers, or partial behavior exist, but the full component is incomplete.
    Partial,
    /// The component is part of the Kinetik vocabulary but is not implemented in this crate yet.
    Planned,
}

impl ComponentConformanceStatus {
    /// Returns a stable display name for the status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "Implemented",
            Self::Partial => "Partial",
            Self::Planned => "Planned",
        }
    }
}

/// Public component taxonomy entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentMetadata {
    /// Public component or pattern name.
    pub name: &'static str,
    /// Stable lower-kebab identifier.
    pub slug: &'static str,
    /// Kinetik-owned category.
    pub category: ComponentCategory,
    /// Honest implementation status.
    pub status: ComponentConformanceStatus,
}

impl ComponentMetadata {
    /// Creates a component taxonomy entry.
    #[must_use]
    pub const fn new(
        name: &'static str,
        slug: &'static str,
        category: ComponentCategory,
        status: ComponentConformanceStatus,
    ) -> Self {
        Self {
            name,
            slug,
            category,
            status,
        }
    }
}

use ComponentCategory::{
    Collection, Control, Display, Docking, Input, Inspector, Overlay, System, TextEditing, Viewport,
};
use ComponentConformanceStatus::{Implemented, Partial, Planned};

/// Data-only registry of Kinetik widget components and editor patterns.
pub const COMPONENT_METADATA: &[ComponentMetadata] = &[
    ComponentMetadata::new("Label", "label", Display, Implemented),
    ComponentMetadata::new("Image", "image", Display, Implemented),
    ComponentMetadata::new("Separator", "separator", Display, Implemented),
    ComponentMetadata::new("Button", "button", Control, Implemented),
    ComponentMetadata::new("IconButton", "icon-button", Control, Implemented),
    ComponentMetadata::new("Checkbox", "checkbox", Input, Implemented),
    ComponentMetadata::new("RadioButton", "radio-button", Input, Implemented),
    ComponentMetadata::new("Toggle", "toggle", Input, Implemented),
    ComponentMetadata::new("Slider", "slider", Input, Implemented),
    ComponentMetadata::new("NumericInput", "numeric-input", Input, Implemented),
    ComponentMetadata::new("TextField", "text-field", TextEditing, Implemented),
    ComponentMetadata::new(
        "MultiLineTextField",
        "multi-line-text-field",
        TextEditing,
        Implemented,
    ),
    ComponentMetadata::new("SearchField", "search-field", TextEditing, Implemented),
    ComponentMetadata::new("List", "list", Collection, Partial),
    ComponentMetadata::new("Grid", "grid", Collection, Partial),
    ComponentMetadata::new("Table", "table", Collection, Partial),
    ComponentMetadata::new("Tree", "tree", Collection, Partial),
    ComponentMetadata::new("PropertyGrid", "property-grid", Inspector, Partial),
    ComponentMetadata::new("Panel", "panel", Docking, Partial),
    ComponentMetadata::new("Frame", "frame", Docking, Partial),
    ComponentMetadata::new("Dock", "dock", Docking, Partial),
    ComponentMetadata::new("Menu", "menu", Overlay, Partial),
    ComponentMetadata::new("MenuItem", "menu-item", Overlay, Partial),
    ComponentMetadata::new("ContextMenu", "context-menu", Overlay, Partial),
    ComponentMetadata::new("Popover", "popover", Overlay, Partial),
    ComponentMetadata::new("Tooltip", "tooltip", Overlay, Partial),
    ComponentMetadata::new("CommandPalette", "command-palette", Overlay, Partial),
    ComponentMetadata::new("Viewport", "viewport", Viewport, Partial),
    ComponentMetadata::new("Ruler", "ruler", Viewport, Partial),
    ComponentMetadata::new("Dropdown", "dropdown", Overlay, Partial),
    ComponentMetadata::new("MenuBar", "menu-bar", Overlay, Partial),
    ComponentMetadata::new("Tabs", "tabs", Docking, Partial),
    ComponentMetadata::new("Toolbar", "toolbar", System, Partial),
    ComponentMetadata::new("StatusBar", "status-bar", System, Partial),
    ComponentMetadata::new("Modal", "modal", Overlay, Planned),
    ComponentMetadata::new("Timeline", "timeline", Viewport, Planned),
    ComponentMetadata::new("TransportControls", "transport-controls", Control, Planned),
    ComponentMetadata::new("ProgressIndicator", "progress-indicator", Display, Planned),
];

/// Looks up component metadata by exact public name.
#[must_use]
pub fn component_metadata(name: &str) -> Option<&'static ComponentMetadata> {
    COMPONENT_METADATA
        .iter()
        .find(|metadata| metadata.name == name)
}

/// Returns all component metadata entries for a category.
pub fn components_by_category(
    category: ComponentCategory,
) -> impl Iterator<Item = &'static ComponentMetadata> {
    COMPONENT_METADATA
        .iter()
        .filter(move |metadata| metadata.category == category)
}
