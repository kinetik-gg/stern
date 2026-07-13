//! Public live property-grid composition contract.

use kinetik_ui_core::{Rect, ScrollResponse, WidgetId};

use crate::collections::ItemId;

use super::{
    PropertyGridAffordanceLayout, PropertyGridLayout, PropertyGridRow, PropertyGridRowRect,
};

/// Configuration for one scrollable property-grid component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyGridConfig {
    /// Deterministic row and column layout.
    pub layout: PropertyGridLayout,
    /// Compact reset/keyframe affordance layout.
    pub affordances: PropertyGridAffordanceLayout,
    /// Extra materialized rows before and after the viewport.
    pub overscan: usize,
    /// Whether the entire grid is unavailable for interaction.
    pub disabled: bool,
}

impl PropertyGridConfig {
    /// Creates an enabled property grid with one overscan row.
    #[must_use]
    pub const fn new(layout: PropertyGridLayout) -> Self {
        Self {
            layout,
            affordances: PropertyGridAffordanceLayout::new(18.0, 4.0),
            overscan: 1,
            disabled: false,
        }
    }

    /// Replaces compact affordance layout.
    #[must_use]
    pub const fn with_affordance_layout(
        mut self,
        affordances: PropertyGridAffordanceLayout,
    ) -> Self {
        self.affordances = affordances;
        self
    }

    /// Replaces visible-range overscan.
    #[must_use]
    pub const fn with_overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self
    }

    /// Sets whether the entire grid is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Default for PropertyGridConfig {
    fn default() -> Self {
        Self::new(PropertyGridLayout::new(24.0, 26.0, 120.0, 6.0, 12.0))
    }
}

/// Effective application access for one live property value cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyGridAccess {
    /// The value may accept editing interaction.
    Editable,
    /// The value remains readable and focusable but must not mutate.
    ReadOnly,
    /// The value is unavailable for interaction.
    Disabled,
}

impl PropertyGridAccess {
    /// Returns true when value controls must reject interaction.
    #[must_use]
    pub const fn disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    /// Returns true when value controls must not mutate application state.
    #[must_use]
    pub const fn read_only(self) -> bool {
        matches!(self, Self::ReadOnly)
    }
}

/// Frozen visible value-cell data passed to the caller callback.
#[derive(Debug, Clone, Copy)]
pub struct PropertyGridCell<'a> {
    root: WidgetId,
    /// Current application-owned row descriptor.
    pub row: &'a PropertyGridRow,
    /// Frozen visible row geometry.
    pub geometry: PropertyGridRowRect,
    /// Value rectangle after reserving reset/keyframe controls.
    pub value_rect: Rect,
    /// Effective grid-plus-row access state.
    pub access: PropertyGridAccess,
}

impl<'a> PropertyGridCell<'a> {
    pub(crate) const fn new(
        root: WidgetId,
        row: &'a PropertyGridRow,
        geometry: PropertyGridRowRect,
        value_rect: Rect,
        access: PropertyGridAccess,
    ) -> Self {
        Self {
            root,
            row,
            geometry,
            value_rect,
            access,
        }
    }

    /// Returns the stable row widget ID.
    #[must_use]
    pub fn row_widget_id(self) -> WidgetId {
        property_grid_row_widget_id(self.root, self.row.id)
    }

    /// Returns the stable value-cell widget ID.
    #[must_use]
    pub fn value_widget_id(self) -> WidgetId {
        property_grid_value_widget_id(self.root, self.row.id)
    }
}

/// Application-owned operation requested by a property-row affordance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyGridIntent {
    /// Reset a property to its application-owned default.
    Reset {
        /// Stable property row identity.
        row: ItemId,
    },
    /// Add or remove a keyframe at the application-owned current time.
    SetKeyed {
        /// Stable property row identity.
        row: ItemId,
        /// Requested keyed state.
        keyed: bool,
    },
}

/// Caller callback result paired with its stable property row.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyGridValueOutput<T> {
    /// Stable property row identity.
    pub row: ItemId,
    /// Caller-produced live value output.
    pub value: T,
}

/// Output from one public property-grid evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyGridOutput<T> {
    /// Stable root widget ID for deriving row and value-cell identities.
    pub root: WidgetId,
    /// Vertical scroll response and retained offset.
    pub scroll: ScrollResponse,
    /// Frozen materialized row geometry.
    pub visible_rows: Vec<PropertyGridRowRect>,
    /// Callback outputs for visible property rows.
    pub values: Vec<PropertyGridValueOutput<T>>,
    /// Ordered application-owned reset/keyframe requests.
    pub intents: Vec<PropertyGridIntent>,
}

/// Returns the stable semantic and control ID for one property-grid row.
#[must_use]
pub fn property_grid_row_widget_id(root: WidgetId, row: ItemId) -> WidgetId {
    root.child(("property-grid-row", row.raw()))
}

/// Returns the stable value-cell ID for one property-grid row.
#[must_use]
pub fn property_grid_value_widget_id(root: WidgetId, row: ItemId) -> WidgetId {
    property_grid_row_widget_id(root, row).child("value")
}
