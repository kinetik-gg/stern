//! Base widget components built from Stern core primitives.

use std::collections::BTreeMap;

use stern_core::{
    Brush, ClipId, Color, ComponentState, CornerRadius, CursorShape, DropTargetResponse,
    FontFeatureToken, ImageId, ImagePrimitive, Insets, Key, KeyState, LinePrimitive, PathElement,
    PathPrimitive, PlatformRequest, Point, Primitive, Rect, RectPrimitive, Response,
    SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticState, SemanticValue,
    Stroke, TextFieldRecipe, TextPrimitive, TextRole, Theme, UiInput, UiMemory, WidgetId,
    draggable, drop_target, fit_box, focusable, pad_rect, pressable, selectable,
};
use stern_text::{
    OrderedTextInputResult, ShapedTextLayout, TextEditMode, TextEditState, TextFeatureSet,
    TextLayoutKey, TextLayoutStore, TextSelection, TextStyle,
};

use crate::{
    IconId,
    inspector::{
        VectorComponentLayout, VectorComponentRect, vector2_component_rects,
        vector3_component_rects, vector4_component_rects,
    },
    overlays::{DropdownModel, DropdownTriggerPresentation},
};

const DEFAULT_SLIDER_STEP_DIVISIONS: f32 = 100.0;
const DEFAULT_SLIDER_PAGE_DIVISIONS: f32 = 10.0;
const DEFAULT_NUMERIC_SCRUB_STEP: f32 = 1.0;
const DEFAULT_NUMERIC_SCRUB_FINE_FACTOR: f32 = 0.1;
const DEFAULT_NUMERIC_SCRUB_COARSE_FACTOR: f32 = 10.0;

mod basic;
mod choice;
mod common;
mod field_helpers;
mod icons;
mod numeric_inputs;
mod search;
mod selector_fields;
mod semantics;
mod slider;
mod surfaces;
mod text_fields;
mod text_geometry;
mod text_interaction;
mod text_support;
mod vector_color_fields;

#[cfg(test)]
mod tests;

pub(crate) use common::{
    ButtonFocusPlacement, RowFocusPlacement, TabFocusPlacement, button_surface_primitives,
    row_surface_primitives, tab_surface_primitives,
};
use common::{
    clicked_select_state, clicked_toggle_state, control_text_origin, label_baseline,
    response_reported_focus, response_reported_pressed, suppress_disabled_interaction_reporting,
    with_hover_cursor, with_response_state,
};
use field_helpers::{field_text_primitive, finite_widget_extent};
use text_support::{
    display_text_with_composition, multi_line_hit_offset, multi_line_text_primitives,
    single_line_hit_offset, single_line_text_primitives, text_field_layout,
    text_input_platform_requests, text_line_fragments,
};

pub use basic::*;
pub use choice::*;
pub use common::WidgetOutput;
pub use icons::*;
pub use numeric_inputs::*;
pub use search::*;
pub use selector_fields::*;
pub use semantics::*;
pub use slider::*;
pub use surfaces::*;
pub use text_fields::*;
pub use vector_color_fields::*;

pub(crate) use numeric_inputs::{
    numeric_input_with_access_runtime, numeric_input_with_text_layouts_and_caret_visibility,
    numeric_scrub_input_with_runtime, numeric_scrub_input_with_text_layouts_and_caret_visibility,
};
pub(crate) use search::{
    search_field_with_access_runtime, search_field_with_text_layouts_and_caret_visibility,
};
pub(crate) use selector_fields::{
    path_field_with_access_runtime, path_field_with_text_layouts_and_caret_visibility,
};
pub(crate) use text_fields::{
    multi_line_text_field_with_access_runtime,
    multi_line_text_field_with_text_layouts_and_caret_visibility, text_field_with_access_runtime,
    text_field_with_access_runtime_and_features, text_field_with_access_runtime_metadata_and_fence,
    text_field_with_pointer_runtime_and_features,
    text_field_with_text_layouts_and_caret_visibility,
    text_field_with_text_layouts_and_caret_visibility_and_ordered_result,
};
pub(crate) use vector_color_fields::{
    vector_scrub_input_with_runtime, vector_scrub_input_with_text_layouts_and_caret_visibility,
};
