//! Inspector and property-grid layout primitives.

mod affordances;
mod layout;
mod row;
mod status;
mod util;
mod vector;

pub use affordances::{
    PropertyGridAffordanceLayout, PropertyGridAffordanceOutput, PropertyGridAffordanceRects,
    property_grid_row_affordance_controls, property_grid_row_affordance_rects,
    property_grid_row_status_semantics,
};
pub use layout::{PropertyGridError, PropertyGridLayout, PropertyGridRowRect};
pub use row::{
    PropertyGridKeyframeAffordance, PropertyGridResetAffordance, PropertyGridRow,
    PropertyGridRowAffordances, PropertyGridRowKind, PropertyGridRowState,
};
pub use status::{
    PropertyGridRowStatus, PropertyGridStatusPresentation, PropertyGridStatusSeverity,
};
pub use vector::{
    VectorComponentLayout, VectorComponentRect, vector2_component_rects, vector3_component_rects,
    vector4_component_rects,
};

#[cfg(test)]
mod tests;
