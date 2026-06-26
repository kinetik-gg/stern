//! Inspector and property-grid layout primitives.

use std::collections::BTreeSet;
use std::ops::Range;

use kinetik_ui_core::{
    Brush, ComponentState, CornerRadius, CursorShape, PlatformRequest, Point, Primitive, Rect,
    RectPrimitive, Response, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole,
    SemanticValue, TextPrimitive, TextRole, Theme, UiInput, UiMemory, WidgetId, focusable,
};

use crate::WidgetOutput;
use crate::collections::ItemId;

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_positive(value: f32) -> Option<f32> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

/// Property-grid row kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyGridRowKind {
    /// Section heading row.
    Section,
    /// Editable property row.
    Property {
        /// Nesting depth for grouped properties.
        depth: usize,
    },
}

/// Validation or help status severity for a property-grid row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PropertyGridStatusSeverity {
    /// No status is attached to the row.
    #[default]
    None,
    /// Informational row status.
    Info,
    /// Non-blocking warning row status.
    Warning,
    /// Blocking error row status.
    Error,
}

/// Deterministic presentation metadata for row validation or help status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyGridStatusPresentation {
    /// Status severity.
    pub severity: PropertyGridStatusSeverity,
    /// Stable compact status label.
    pub label: &'static str,
    /// True when the row should show a status accent.
    pub accented: bool,
    /// True when the status should be treated as blocking validation.
    pub blocking: bool,
}

impl PropertyGridStatusSeverity {
    /// Returns deterministic presentation metadata for this severity.
    #[must_use]
    pub const fn presentation(self) -> PropertyGridStatusPresentation {
        match self {
            Self::None => PropertyGridStatusPresentation {
                severity: self,
                label: "None",
                accented: false,
                blocking: false,
            },
            Self::Info => PropertyGridStatusPresentation {
                severity: self,
                label: "Info",
                accented: true,
                blocking: false,
            },
            Self::Warning => PropertyGridStatusPresentation {
                severity: self,
                label: "Warning",
                accented: true,
                blocking: false,
            },
            Self::Error => PropertyGridStatusPresentation {
                severity: self,
                label: "Error",
                accented: true,
                blocking: true,
            },
        }
    }
}

/// Data-only validation or help status for a property-grid row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PropertyGridRowStatus {
    /// Status severity.
    pub severity: PropertyGridStatusSeverity,
    /// Optional status message owned by the application.
    pub message: Option<String>,
}

impl PropertyGridRowStatus {
    /// Creates a row status with the given severity and no message.
    #[must_use]
    pub const fn severity(severity: PropertyGridStatusSeverity) -> Self {
        Self {
            severity,
            message: None,
        }
    }

    /// Creates an informational row status.
    #[must_use]
    pub fn info(message: impl Into<String>) -> Self {
        Self::severity(PropertyGridStatusSeverity::Info).with_message(message)
    }

    /// Creates a warning row status.
    #[must_use]
    pub fn warning(message: impl Into<String>) -> Self {
        Self::severity(PropertyGridStatusSeverity::Warning).with_message(message)
    }

    /// Creates an error row status.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self::severity(PropertyGridStatusSeverity::Error).with_message(message)
    }

    /// Returns this status with an attached message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Returns true when this status represents a blocking error.
    #[must_use]
    pub const fn is_blocking_error(&self) -> bool {
        matches!(self.severity, PropertyGridStatusSeverity::Error)
    }

    /// Returns deterministic presentation metadata for this status.
    #[must_use]
    pub const fn presentation(&self) -> PropertyGridStatusPresentation {
        self.severity.presentation()
    }

    /// Returns accessible status text including severity and message when present.
    #[must_use]
    pub fn semantic_text(&self) -> Option<String> {
        let presentation = self.presentation();
        if matches!(presentation.severity, PropertyGridStatusSeverity::None) {
            return None;
        }

        Some(match self.message.as_deref() {
            Some(message) if !message.is_empty() => format!("{}: {message}", presentation.label),
            _ => presentation.label.to_owned(),
        })
    }
}

/// Reset-to-default affordance metadata for a property-grid row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PropertyGridResetAffordance {
    /// True when a reset control should be presented.
    pub available: bool,
    /// True when the current value already matches the application-owned default.
    pub at_default: bool,
}

impl PropertyGridResetAffordance {
    /// Creates reset affordance metadata.
    #[must_use]
    pub const fn new(available: bool, at_default: bool) -> Self {
        Self {
            available,
            at_default,
        }
    }
}

/// Keyframe affordance metadata for a property-grid row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PropertyGridKeyframeAffordance {
    /// True when a keyframe control should be presented.
    pub available: bool,
    /// True when the current property is keyed at the current application time.
    pub keyed: bool,
}

impl PropertyGridKeyframeAffordance {
    /// Creates keyframe affordance metadata.
    #[must_use]
    pub const fn new(available: bool, keyed: bool) -> Self {
        Self { available, keyed }
    }
}

/// App-owned property affordance metadata attached to a property-grid row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PropertyGridRowAffordances {
    /// Reset-to-default control metadata.
    pub reset: PropertyGridResetAffordance,
    /// Keyframe toggle control metadata.
    pub keyframe: PropertyGridKeyframeAffordance,
}

impl PropertyGridRowAffordances {
    /// Creates neutral affordance metadata.
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            reset: PropertyGridResetAffordance::new(false, false),
            keyframe: PropertyGridKeyframeAffordance::new(false, false),
        }
    }

    /// Returns this metadata with reset-to-default state set.
    #[must_use]
    pub const fn with_reset(mut self, available: bool, at_default: bool) -> Self {
        self.reset = PropertyGridResetAffordance::new(available, at_default);
        self
    }

    /// Returns this metadata with keyframe state set.
    #[must_use]
    pub const fn with_keyframe(mut self, available: bool, keyed: bool) -> Self {
        self.keyframe = PropertyGridKeyframeAffordance::new(available, keyed);
        self
    }
}

/// Data-only form state metadata for a property-grid row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PropertyGridRowState {
    /// True when the row should not accept interaction.
    pub disabled: bool,
    /// True when the row value should be presented as non-editable.
    pub read_only: bool,
    /// True when the row represents a required property.
    pub required: bool,
    /// Optional help text owned by the application.
    pub help_text: Option<String>,
    /// Optional validation or help status owned by the application.
    pub status: PropertyGridRowStatus,
    /// Optional reset/keyframe affordance metadata owned by the application.
    pub affordances: PropertyGridRowAffordances,
}

impl PropertyGridRowState {
    /// Creates neutral row state metadata.
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            disabled: false,
            read_only: false,
            required: false,
            help_text: None,
            status: PropertyGridRowStatus::severity(PropertyGridStatusSeverity::None),
            affordances: PropertyGridRowAffordances::neutral(),
        }
    }

    /// Returns this metadata with disabled state set.
    #[must_use]
    pub const fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Returns this metadata with read-only state set.
    #[must_use]
    pub const fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Returns this metadata with required state set.
    #[must_use]
    pub const fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Returns this metadata with help text attached.
    #[must_use]
    pub fn with_help_text(mut self, help_text: impl Into<String>) -> Self {
        self.help_text = Some(help_text.into());
        self
    }

    /// Returns this metadata with status attached.
    #[must_use]
    pub fn with_status(mut self, status: PropertyGridRowStatus) -> Self {
        self.status = status;
        self
    }

    /// Returns this metadata with reset-to-default affordance state set.
    #[must_use]
    pub const fn with_resettable(mut self, available: bool, at_default: bool) -> Self {
        self.affordances = self.affordances.with_reset(available, at_default);
        self
    }

    /// Returns this metadata with keyframe affordance state set.
    #[must_use]
    pub const fn with_keyframeable(mut self, available: bool, keyed: bool) -> Self {
        self.affordances = self.affordances.with_keyframe(available, keyed);
        self
    }

    /// Returns true when this metadata carries a blocking error status.
    #[must_use]
    pub const fn has_blocking_error(&self) -> bool {
        self.status.is_blocking_error()
    }
}

/// One property-grid row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyGridRow {
    /// Stable row identity.
    pub id: ItemId,
    /// User-visible row label.
    pub label: String,
    /// Row kind.
    pub kind: PropertyGridRowKind,
    /// Data-only row state metadata.
    pub state: PropertyGridRowState,
}

impl PropertyGridRow {
    /// Creates a section heading row.
    #[must_use]
    pub fn section(id: ItemId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            kind: PropertyGridRowKind::Section,
            state: PropertyGridRowState::neutral(),
        }
    }

    /// Creates an editable property row.
    #[must_use]
    pub fn property(id: ItemId, label: impl Into<String>, depth: usize) -> Self {
        Self {
            id,
            label: label.into(),
            kind: PropertyGridRowKind::Property { depth },
            state: PropertyGridRowState::neutral(),
        }
    }

    /// Returns this row with state metadata attached.
    #[must_use]
    pub fn with_state(mut self, state: PropertyGridRowState) -> Self {
        self.state = state;
        self
    }

    /// Returns this row with disabled state set.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.state = self.state.with_disabled(disabled);
        self
    }

    /// Returns this row with read-only state set.
    #[must_use]
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.state = self.state.with_read_only(read_only);
        self
    }

    /// Returns this row with required state set.
    #[must_use]
    pub fn with_required(mut self, required: bool) -> Self {
        self.state = self.state.with_required(required);
        self
    }

    /// Returns this row with help text attached.
    #[must_use]
    pub fn with_help_text(mut self, help_text: impl Into<String>) -> Self {
        self.state = self.state.with_help_text(help_text);
        self
    }

    /// Returns this row with status attached.
    #[must_use]
    pub fn with_status(mut self, status: PropertyGridRowStatus) -> Self {
        self.state = self.state.with_status(status);
        self
    }

    /// Returns this row with reset-to-default affordance state set.
    #[must_use]
    pub fn with_resettable(mut self, available: bool, at_default: bool) -> Self {
        self.state = self.state.with_resettable(available, at_default);
        self
    }

    /// Returns this row with keyframe affordance state set.
    #[must_use]
    pub fn with_keyframeable(mut self, available: bool, keyed: bool) -> Self {
        self.state = self.state.with_keyframeable(available, keyed);
        self
    }

    /// Returns true when this row can accept interaction.
    #[must_use]
    pub fn is_interactable(&self) -> bool {
        matches!(self.kind, PropertyGridRowKind::Property { .. }) && !self.state.disabled
    }

    /// Returns true when this row represents an editable property value.
    #[must_use]
    pub fn is_editable(&self) -> bool {
        self.is_interactable() && !self.state.read_only
    }

    /// Returns true when this row carries a blocking error status.
    #[must_use]
    pub fn has_blocking_error(&self) -> bool {
        self.state.has_blocking_error()
    }

    /// Returns true when this row can emit a reset-to-default request.
    #[must_use]
    pub fn can_request_reset(&self) -> bool {
        matches!(self.kind, PropertyGridRowKind::Property { .. })
            && !self.state.disabled
            && !self.state.read_only
            && self.state.affordances.reset.available
            && !self.state.affordances.reset.at_default
    }

    /// Returns true when this row can emit a keyframe toggle request.
    #[must_use]
    pub fn can_request_keyframe_toggle(&self) -> bool {
        matches!(self.kind, PropertyGridRowKind::Property { .. })
            && !self.state.disabled
            && !self.state.read_only
            && self.state.affordances.keyframe.available
    }
}

/// Layout tuning for compact property-row affordance controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyGridAffordanceLayout {
    /// Square control size for reset and keyframe controls.
    pub button_size: f32,
    /// Gap between controls and the value editor.
    pub gap: f32,
    /// Minimum value-editor width preserved before controls are shown.
    pub min_value_width: f32,
}

impl PropertyGridAffordanceLayout {
    /// Creates property affordance layout tuning.
    #[must_use]
    pub const fn new(button_size: f32, gap: f32) -> Self {
        Self {
            button_size,
            gap,
            min_value_width: 40.0,
        }
    }

    /// Sets the minimum value-editor width preserved before controls are shown.
    #[must_use]
    pub const fn with_min_value_width(mut self, min_value_width: f32) -> Self {
        self.min_value_width = min_value_width;
        self
    }
}

impl Default for PropertyGridAffordanceLayout {
    fn default() -> Self {
        Self::new(18.0, 4.0)
    }
}

/// Rectangles assigned to property-row value and affordance controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyGridAffordanceRects {
    /// Value/control rectangle after reserving visible affordance controls.
    pub value_rect: Rect,
    /// Reset-to-default control rectangle, when visible.
    pub reset_rect: Option<Rect>,
    /// Keyframe toggle control rectangle, when visible.
    pub keyframe_rect: Option<Rect>,
}

/// Output from property-row affordance controls.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyGridAffordanceOutput {
    /// Widget primitives, semantics, and platform requests emitted by controls.
    pub widget: WidgetOutput,
    /// True when the user requested an app-owned reset-to-default operation.
    pub reset_requested: bool,
    /// True when the user requested an app-owned keyframe toggle operation.
    pub keyframe_toggle_requested: bool,
    /// Keyed state requested by the keyframe toggle, without storing animation data.
    pub requested_keyed: bool,
    /// Reset control response, when visible.
    pub reset_response: Option<Response>,
    /// Keyframe control response, when visible.
    pub keyframe_response: Option<Response>,
}

/// Computes compact reset/keyframe affordance rectangles for one value cell.
#[must_use]
pub fn property_grid_row_affordance_rects(
    row: &PropertyGridRow,
    value_rect: Rect,
    layout: PropertyGridAffordanceLayout,
) -> PropertyGridAffordanceRects {
    let button_size = finite_non_negative(layout.button_size)
        .min(finite_non_negative(value_rect.height))
        .min(finite_non_negative(value_rect.width));
    let gap = finite_non_negative(layout.gap).min(finite_non_negative(value_rect.width));
    let min_value_width = finite_non_negative(layout.min_value_width);
    let mut cursor = value_rect.max_x();

    let keyframe_rect = if row.state.affordances.keyframe.available
        && can_reserve_affordance(cursor, value_rect.x, button_size, gap, min_value_width)
    {
        cursor -= button_size;
        let rect = Rect::new(
            cursor,
            value_rect.y + (value_rect.height - button_size).max(0.0) * 0.5,
            button_size,
            button_size,
        );
        cursor -= gap.min((cursor - value_rect.x).max(0.0));
        Some(rect)
    } else {
        None
    };

    let reset_rect = if row.state.affordances.reset.available
        && can_reserve_affordance(cursor, value_rect.x, button_size, gap, min_value_width)
    {
        cursor -= button_size.min((cursor - value_rect.x).max(0.0));
        let width = button_size.min((value_rect.max_x() - cursor).max(0.0));
        let rect = Rect::new(
            cursor,
            value_rect.y + (value_rect.height - button_size).max(0.0) * 0.5,
            width,
            button_size,
        );
        cursor -= gap.min((cursor - value_rect.x).max(0.0));
        Some(rect)
    } else {
        None
    };

    PropertyGridAffordanceRects {
        value_rect: Rect::new(
            value_rect.x,
            value_rect.y,
            (cursor - value_rect.x).max(0.0),
            value_rect.height,
        ),
        reset_rect,
        keyframe_rect,
    }
}

fn can_reserve_affordance(
    cursor: f32,
    value_x: f32,
    button_size: f32,
    gap: f32,
    min_value_width: f32,
) -> bool {
    button_size > 0.0 && cursor - value_x >= button_size + gap + min_value_width
}

/// Emits compact property-row reset and keyframe controls.
#[must_use]
pub fn property_grid_row_affordance_controls(
    id: WidgetId,
    row: &PropertyGridRow,
    rects: PropertyGridAffordanceRects,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
) -> PropertyGridAffordanceOutput {
    let mut widget = WidgetOutput::new(None, Vec::new());
    let mut reset_response = None;
    let mut keyframe_response = None;
    let mut reset_requested = false;
    let mut keyframe_toggle_requested = false;
    let mut requested_keyed = row.state.affordances.keyframe.keyed;

    if let Some(rect) = rects.reset_rect {
        let disabled = !row.can_request_reset();
        let response = affordance_button(
            &mut widget,
            id.child("reset"),
            rect,
            format!("Reset {} to default", row.label),
            "Reset",
            "R",
            false,
            disabled,
            input,
            memory,
            theme,
        );
        reset_requested = !disabled && response.clicked;
        reset_response = Some(response);
    }

    if let Some(rect) = rects.keyframe_rect {
        let disabled = !row.can_request_keyframe_toggle();
        let response = affordance_button(
            &mut widget,
            id.child("keyframe"),
            rect,
            format!("Toggle keyframe for {}", row.label),
            "Toggle keyframe",
            "K",
            row.state.affordances.keyframe.keyed,
            disabled,
            input,
            memory,
            theme,
        );
        keyframe_toggle_requested = !disabled && response.clicked;
        if keyframe_toggle_requested {
            requested_keyed = !row.state.affordances.keyframe.keyed;
        }
        keyframe_response = Some(response);
    }

    PropertyGridAffordanceOutput {
        widget,
        reset_requested,
        keyframe_toggle_requested,
        requested_keyed,
        reset_response,
        keyframe_response,
    }
}

/// Builds deterministic semantic metadata for a property-grid row status.
#[must_use]
pub fn property_grid_row_status_semantics(
    id: WidgetId,
    row: &PropertyGridRow,
    row_rect: PropertyGridRowRect,
) -> Option<SemanticNode> {
    let status_text = row.state.status.semantic_text()?;
    let mut node = SemanticNode::new(id.child("status"), SemanticRole::Label, row_rect.rect)
        .with_label(format!("{} status", row.label));
    node.description = Some(status_text.clone());
    node.state.value = Some(SemanticValue::Text(status_text));
    node.state.disabled = row.state.disabled;
    Some(node)
}

#[allow(clippy::too_many_arguments)]
fn affordance_button(
    widget: &mut WidgetOutput,
    id: WidgetId,
    rect: Rect,
    label: String,
    action_label: &'static str,
    glyph: &'static str,
    selected: bool,
    disabled: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
) -> Response {
    let mut response = focusable(id, rect, input, memory, disabled);
    suppress_disabled_interaction_reporting(&mut response);
    response.state.selected = selected;
    let recipe = theme.button(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed && !response.state.disabled,
        focused: response.state.focused && !response.state.disabled,
        disabled,
        selected,
    });

    widget.primitives.push(Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius: CornerRadius::all(3.0),
    }));
    widget.primitives.push(Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(
            rect.x + (rect.width * 0.5 - theme.font(TextRole::Label).size * 0.28).max(1.0),
            rect.y
                + (rect.height - theme.font(TextRole::Label).line_height).max(0.0) * 0.5
                + theme.font(TextRole::Label).size,
        ),
        text: glyph.to_owned(),
        family: theme.font(TextRole::Label).family.to_owned(),
        size: theme.font(TextRole::Label).size,
        line_height: theme.font(TextRole::Label).line_height,
        brush: Brush::Solid(recipe.foreground),
    }));

    let mut node = SemanticNode::new(id, SemanticRole::IconButton, rect)
        .with_label(label)
        .focusable(!disabled);
    node.state.disabled = disabled;
    node.state.focused = response.state.focused && !response.state.disabled;
    node.state.pressed = response.state.pressed && !response.state.disabled;
    node.state.selected = selected;
    if !disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Invoke,
            action_label,
        ));
    }
    widget.semantics.push(node);

    if response.state.hovered && !response.state.disabled {
        widget
            .platform_requests
            .push(PlatformRequest::SetCursor(CursorShape::PointingHand));
    }

    response
}

fn suppress_disabled_interaction_reporting(response: &mut Response) {
    if response.state.disabled {
        response.state.focused = false;
        response.state.active = false;
        response.state.pressed = false;
    }
}

/// Property-grid structural error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyGridError {
    /// More than one row uses the same ID.
    DuplicateRowId {
        /// Duplicated row identity.
        id: ItemId,
    },
}

/// Rectangle assigned to one property-grid row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyGridRowRect {
    /// Source row index.
    pub index: usize,
    /// Stable row identity.
    pub id: ItemId,
    /// Row kind.
    pub kind: PropertyGridRowKind,
    /// Full row rectangle.
    pub rect: Rect,
    /// Label or section-title rectangle.
    pub label_rect: Rect,
    /// Value/control rectangle.
    pub value_rect: Rect,
}

/// Layout tuning for compact vector property fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VectorComponentLayout {
    /// Gap between vector component groups.
    pub component_gap: f32,
    /// Width reserved for compact component labels such as X/Y/Z/W.
    pub label_width: f32,
    /// Gap between a compact component label and its value field.
    pub label_gap: f32,
    /// Preferred minimum value-field width before labels are compressed.
    pub min_value_width: f32,
}

impl VectorComponentLayout {
    /// Creates a vector component layout.
    #[must_use]
    pub const fn new(
        component_gap: f32,
        label_width: f32,
        label_gap: f32,
        min_value_width: f32,
    ) -> Self {
        Self {
            component_gap,
            label_width,
            label_gap,
            min_value_width,
        }
    }
}

impl Default for VectorComponentLayout {
    fn default() -> Self {
        Self::new(6.0, 10.0, 3.0, 24.0)
    }
}

/// Rectangles assigned to one vector component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VectorComponentRect {
    /// Component index.
    pub index: usize,
    /// Compact component label.
    pub label: &'static str,
    /// Full component group rectangle.
    pub rect: Rect,
    /// Compact label rectangle.
    pub label_rect: Rect,
    /// Numeric value field rectangle.
    pub value_rect: Rect,
}

/// Computes deterministic Vec2 component rectangles.
#[must_use]
pub fn vector2_component_rects(
    rect: Rect,
    layout: VectorComponentLayout,
) -> [VectorComponentRect; 2] {
    vector_component_rects(rect, ["X", "Y"], layout)
}

/// Computes deterministic Vec3 component rectangles.
#[must_use]
pub fn vector3_component_rects(
    rect: Rect,
    layout: VectorComponentLayout,
) -> [VectorComponentRect; 3] {
    vector_component_rects(rect, ["X", "Y", "Z"], layout)
}

/// Computes deterministic Vec4 component rectangles.
#[must_use]
pub fn vector4_component_rects(
    rect: Rect,
    layout: VectorComponentLayout,
) -> [VectorComponentRect; 4] {
    vector_component_rects(rect, ["X", "Y", "Z", "W"], layout)
}

#[allow(clippy::cast_precision_loss)]
fn vector_component_rects<const N: usize>(
    rect: Rect,
    labels: [&'static str; N],
    layout: VectorComponentLayout,
) -> [VectorComponentRect; N] {
    let count = N.max(1) as f32;
    let width = finite_non_negative(rect.width);
    let height = finite_non_negative(rect.height);
    let sanitized_component_gap = finite_non_negative(layout.component_gap);
    let total_gap = (sanitized_component_gap * (count - 1.0)).min(width);
    let component_width = (width - total_gap).max(0.0) / count;
    let preferred_label_width = finite_non_negative(layout.label_width);
    let preferred_label_gap = finite_non_negative(layout.label_gap);
    let min_value_width = finite_non_negative(layout.min_value_width);

    std::array::from_fn(|index| {
        let x = rect.x + index as f32 * (component_width + sanitized_component_gap);
        let component_rect = Rect::new(x, rect.y, component_width, height);
        let label_fits =
            component_width >= preferred_label_width + preferred_label_gap + min_value_width;
        let label_width = if label_fits {
            preferred_label_width.min(component_width)
        } else {
            (component_width * 0.35).min(preferred_label_width).max(0.0)
        };
        let label_gap = if component_width > label_width {
            preferred_label_gap.min(component_width - label_width)
        } else {
            0.0
        };
        let value_x = x + label_width + label_gap;
        let value_width = (component_rect.max_x() - value_x).max(0.0);

        VectorComponentRect {
            index,
            label: labels[index],
            rect: component_rect,
            label_rect: Rect::new(x, rect.y, label_width, height),
            value_rect: Rect::new(value_x, rect.y, value_width, height),
        }
    })
}

/// Inspector-style property-grid layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyGridLayout {
    /// Regular property row height.
    pub row_height: f32,
    /// Section heading row height.
    pub section_height: f32,
    /// Preferred label column width.
    pub label_width: f32,
    /// Gap between label and value columns.
    pub column_gap: f32,
    /// Per-depth indentation.
    pub indent_width: f32,
}

impl PropertyGridLayout {
    /// Creates a property-grid layout.
    #[must_use]
    pub const fn new(
        row_height: f32,
        section_height: f32,
        label_width: f32,
        column_gap: f32,
        indent_width: f32,
    ) -> Self {
        Self {
            row_height,
            section_height,
            label_width,
            column_gap,
            indent_width,
        }
    }

    /// Returns the sanitized property row height.
    #[must_use]
    pub fn effective_row_height(self) -> Option<f32> {
        finite_positive(self.row_height)
    }

    /// Returns the sanitized section heading height.
    #[must_use]
    pub fn effective_section_height(self) -> Option<f32> {
        finite_positive(self.section_height)
    }

    /// Returns the sanitized label column width.
    #[must_use]
    pub fn effective_label_width(self) -> f32 {
        finite_non_negative(self.label_width)
    }

    /// Returns the sanitized gap between label and value columns.
    #[must_use]
    pub fn effective_column_gap(self) -> f32 {
        finite_non_negative(self.column_gap)
    }

    /// Returns the sanitized per-depth indentation.
    #[must_use]
    pub fn effective_indent_width(self) -> f32 {
        finite_non_negative(self.indent_width)
    }

    /// Validates row identity invariants.
    ///
    /// # Errors
    ///
    /// Returns [`PropertyGridError`] when duplicate row IDs are present.
    pub fn validate_rows(rows: &[PropertyGridRow]) -> Result<(), PropertyGridError> {
        let mut ids = BTreeSet::new();
        for row in rows {
            if !ids.insert(row.id) {
                return Err(PropertyGridError::DuplicateRowId { id: row.id });
            }
        }
        Ok(())
    }

    /// Computes the height for one row kind.
    #[must_use]
    pub fn row_extent(self, kind: PropertyGridRowKind) -> f32 {
        match kind {
            PropertyGridRowKind::Section => self.effective_section_height(),
            PropertyGridRowKind::Property { .. } => self.effective_row_height(),
        }
        .unwrap_or(0.0)
    }

    /// Computes total content height.
    #[must_use]
    pub fn content_height(self, rows: &[PropertyGridRow]) -> f32 {
        rows.iter()
            .map(|row| self.row_extent(row.kind))
            .sum::<f32>()
    }

    /// Computes the maximum vertical scroll offset.
    #[must_use]
    pub fn max_scroll_offset(self, rows: &[PropertyGridRow], viewport_height: f32) -> f32 {
        (self.content_height(rows) - finite_non_negative(viewport_height)).max(0.0)
    }

    /// Clamps a vertical scroll offset to the valid range.
    #[must_use]
    pub fn clamp_scroll_offset(
        self,
        rows: &[PropertyGridRow],
        viewport_height: f32,
        scroll_offset: f32,
    ) -> f32 {
        finite_non_negative(scroll_offset).min(self.max_scroll_offset(rows, viewport_height))
    }

    /// Computes visible row indexes for a viewport.
    #[must_use]
    pub fn visible_range(
        self,
        rows: &[PropertyGridRow],
        scroll_offset: f32,
        viewport_height: f32,
        overscan: usize,
    ) -> Range<usize> {
        let Some(viewport_height) = finite_positive(viewport_height) else {
            return 0..0;
        };
        if rows.is_empty() {
            return 0..0;
        }
        if self.content_height(rows) <= 0.0 {
            return 0..0;
        }

        let scroll_offset = self.clamp_scroll_offset(rows, viewport_height, scroll_offset);
        let viewport_end = scroll_offset + viewport_height;
        let mut y = 0.0;
        let mut start = None;
        let mut end = rows.len();

        for (index, row) in rows.iter().enumerate() {
            let height = self.row_extent(row.kind);
            let row_end = y + height;
            if start.is_none() && row_end > scroll_offset {
                start = Some(index);
            }
            if y >= viewport_end {
                end = index;
                break;
            }
            y = row_end;
        }

        let start = start.unwrap_or(rows.len()).saturating_sub(overscan);
        let end = end.saturating_add(overscan).min(rows.len());
        start..end
    }

    /// Computes row rectangles in viewport coordinates.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn visible_row_rects(
        self,
        bounds: Rect,
        rows: &[PropertyGridRow],
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<PropertyGridRowRect> {
        let scroll_offset = self.clamp_scroll_offset(rows, bounds.height, scroll_offset);
        let visible = self.visible_range(rows, scroll_offset, bounds.height, overscan);
        let mut y = bounds.y - scroll_offset;
        for row in rows.iter().take(visible.start) {
            y += self.row_extent(row.kind);
        }

        visible
            .map(|index| {
                let row = &rows[index];
                let height = self.row_extent(row.kind);
                let rect = Rect::new(
                    bounds.x,
                    y,
                    finite_non_negative(bounds.width),
                    finite_non_negative(height),
                );
                y += height;
                self.row_rect(index, row, rect)
            })
            .collect()
    }

    #[allow(clippy::cast_precision_loss)]
    fn row_rect(self, index: usize, row: &PropertyGridRow, rect: Rect) -> PropertyGridRowRect {
        match row.kind {
            PropertyGridRowKind::Section => PropertyGridRowRect {
                index,
                id: row.id,
                kind: row.kind,
                rect,
                label_rect: rect,
                value_rect: Rect::new(rect.max_x(), rect.y, 0.0, rect.height),
            },
            PropertyGridRowKind::Property { depth } => {
                let indent = depth as f32 * self.effective_indent_width();
                let x = rect.x + indent;
                let available = (rect.width - indent).max(0.0);
                let label_width = self.effective_label_width().min(available);
                let gap = if available > label_width {
                    self.effective_column_gap().min(available - label_width)
                } else {
                    0.0
                };
                let value_x = x + label_width + gap;
                let value_width = (rect.max_x() - value_x).max(0.0);
                PropertyGridRowRect {
                    index,
                    id: row.id,
                    kind: row.kind,
                    rect,
                    label_rect: Rect::new(x, rect.y, label_width, rect.height),
                    value_rect: Rect::new(value_x, rect.y, value_width, rect.height),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PropertyGridAffordanceLayout, PropertyGridError, PropertyGridLayout, PropertyGridRow,
        PropertyGridRowAffordances, PropertyGridRowState, PropertyGridRowStatus,
        PropertyGridStatusSeverity, VectorComponentLayout, VectorComponentRect,
        property_grid_row_affordance_controls, property_grid_row_affordance_rects,
        property_grid_row_status_semantics, vector2_component_rects, vector3_component_rects,
        vector4_component_rects,
    };
    use crate::ItemId;
    use kinetik_ui_core::{
        Point, PointerButtonState, PointerInput, Rect, SemanticActionKind, SemanticRole,
        SemanticValue, UiInput, UiMemory, WidgetId, default_dark_theme,
    };

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }

    fn assert_rect_finite(rect: Rect) {
        assert!(rect.x.is_finite(), "rect x must be finite: {rect:?}");
        assert!(rect.y.is_finite(), "rect y must be finite: {rect:?}");
        assert!(
            rect.width.is_finite(),
            "rect width must be finite: {rect:?}"
        );
        assert!(
            rect.height.is_finite(),
            "rect height must be finite: {rect:?}"
        );
    }

    fn assert_vector_components_finite_and_non_overlapping(components: &[VectorComponentRect]) {
        for component in components {
            assert_rect_finite(component.rect);
            assert_rect_finite(component.label_rect);
            assert_rect_finite(component.value_rect);
            assert!(component.label_rect.max_x() <= component.value_rect.x);
            assert!(component.value_rect.max_x() <= component.rect.max_x());
        }

        for pair in components.windows(2) {
            assert!(pair[0].rect.max_x() <= pair[1].rect.x);
        }
    }

    fn rows() -> Vec<PropertyGridRow> {
        vec![
            PropertyGridRow::section(ItemId::from_raw(1), "Transform"),
            PropertyGridRow::property(ItemId::from_raw(2), "Position", 0),
            PropertyGridRow::property(ItemId::from_raw(3), "X", 1),
            PropertyGridRow::property(ItemId::from_raw(4), "Y", 1),
        ]
    }

    fn pointer_input(x: f32, y: f32, down: bool, pressed: bool, released: bool) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(Point::new(x, y)),
                primary: PointerButtonState::new(down, pressed, released),
                ..PointerInput::default()
            },
            ..UiInput::default()
        }
    }

    #[test]
    fn property_grid_validates_duplicate_row_ids() {
        let rows = vec![
            PropertyGridRow::property(ItemId::from_raw(1), "A", 0)
                .with_status(PropertyGridRowStatus::warning("Check value")),
            PropertyGridRow::property(ItemId::from_raw(1), "B", 0)
                .with_disabled(true)
                .with_required(true),
        ];

        assert_eq!(
            PropertyGridLayout::validate_rows(&rows),
            Err(PropertyGridError::DuplicateRowId {
                id: ItemId::from_raw(1)
            })
        );
    }

    #[test]
    fn property_grid_row_metadata_defaults_to_neutral_state() {
        let section = PropertyGridRow::section(ItemId::from_raw(1), "Transform");
        let property = PropertyGridRow::property(ItemId::from_raw(2), "Position", 0);

        assert_eq!(section.state, PropertyGridRowState::neutral());
        assert_eq!(property.state, PropertyGridRowState::neutral());
        assert!(!section.is_interactable());
        assert!(!section.is_editable());
        assert!(property.is_interactable());
        assert!(property.is_editable());
        assert!(!property.has_blocking_error());
        assert_eq!(
            property.state.affordances,
            PropertyGridRowAffordances::neutral()
        );
        assert!(!property.can_request_reset());
        assert!(!property.can_request_keyframe_toggle());
    }

    #[test]
    fn property_grid_row_builder_attaches_state_metadata() {
        let row = PropertyGridRow::property(ItemId::from_raw(1), "Exposure", 0)
            .with_disabled(true)
            .with_read_only(true)
            .with_required(true)
            .with_help_text("Use scene-referred values")
            .with_status(PropertyGridRowStatus::warning(
                "Value is above preview range",
            ))
            .with_resettable(true, false)
            .with_keyframeable(true, true);

        assert!(row.state.disabled);
        assert!(row.state.read_only);
        assert!(row.state.required);
        assert_eq!(
            row.state.help_text.as_deref(),
            Some("Use scene-referred values")
        );
        assert_eq!(
            row.state.status.severity,
            PropertyGridStatusSeverity::Warning
        );
        assert_eq!(
            row.state.status.message.as_deref(),
            Some("Value is above preview range")
        );
        assert_eq!(
            row.state.affordances,
            PropertyGridRowAffordances::neutral()
                .with_reset(true, false)
                .with_keyframe(true, true)
        );
        assert!(!row.is_interactable());
        assert!(!row.is_editable());
        assert!(!row.has_blocking_error());
        assert!(!row.can_request_reset());
        assert!(!row.can_request_keyframe_toggle());
    }

    #[test]
    fn property_grid_row_helpers_reflect_editability_and_error_state() {
        let read_only =
            PropertyGridRow::property(ItemId::from_raw(1), "Script", 0).with_read_only(true);
        let disabled =
            PropertyGridRow::property(ItemId::from_raw(2), "Collider", 0).with_disabled(true);
        let error = PropertyGridRow::property(ItemId::from_raw(3), "Mass", 0)
            .with_status(PropertyGridRowStatus::error("Mass must be positive"));
        let info = PropertyGridRow::property(ItemId::from_raw(4), "Material", 0)
            .with_status(PropertyGridRowStatus::info("Inherited from parent"));

        assert!(read_only.is_interactable());
        assert!(!read_only.is_editable());
        assert!(!read_only.has_blocking_error());
        assert!(!disabled.is_interactable());
        assert!(!disabled.is_editable());
        assert!(error.is_interactable());
        assert!(error.is_editable());
        assert!(error.has_blocking_error());
        assert!(!info.has_blocking_error());
    }

    #[test]
    fn property_grid_computes_content_and_scroll_extents() {
        let rows = rows();
        let layout = PropertyGridLayout::new(20.0, 24.0, 90.0, 8.0, 12.0);

        assert_approx(layout.content_height(&rows), 84.0);
        assert_approx(layout.max_scroll_offset(&rows, 44.0), 40.0);
        assert_approx(layout.clamp_scroll_offset(&rows, 44.0, 500.0), 40.0);
        assert_eq!(layout.visible_range(&rows, 20.0, 44.0, 0), 0..3);
        assert_eq!(layout.visible_range(&rows, 44.0, 20.0, 0), 2..3);
    }

    #[test]
    fn property_grid_assigns_section_label_and_value_rects() {
        let rows = rows();
        let layout = PropertyGridLayout::new(20.0, 24.0, 90.0, 8.0, 12.0);
        let rects = layout.visible_row_rects(Rect::new(10.0, 100.0, 220.0, 84.0), &rows, 0.0, 0);

        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0].id, ItemId::from_raw(1));
        assert_eq!(rects[0].label_rect, rects[0].rect);
        assert_approx(rects[1].label_rect.x, 10.0);
        assert_approx(rects[1].label_rect.width, 90.0);
        assert_approx(rects[1].value_rect.x, 108.0);
        assert_approx(rects[2].label_rect.x, 22.0);
        assert_approx(rects[2].value_rect.x, 120.0);
    }

    #[test]
    fn property_grid_metadata_does_not_change_row_rectangles() {
        let plain = rows();
        let annotated = vec![
            PropertyGridRow::section(ItemId::from_raw(1), "Transform")
                .with_help_text("Object transform"),
            PropertyGridRow::property(ItemId::from_raw(2), "Position", 0)
                .with_required(true)
                .with_status(PropertyGridRowStatus::severity(
                    PropertyGridStatusSeverity::Info,
                )),
            PropertyGridRow::property(ItemId::from_raw(3), "X", 1)
                .with_status(PropertyGridRowStatus::warning("Outside guide range")),
            PropertyGridRow::property(ItemId::from_raw(4), "Y", 1)
                .with_read_only(true)
                .with_status(PropertyGridRowStatus::error("Missing linked property")),
        ];
        let layout = PropertyGridLayout::new(20.0, 24.0, 90.0, 8.0, 12.0);
        let bounds = Rect::new(10.0, 100.0, 220.0, 84.0);

        assert_eq!(
            layout.visible_row_rects(bounds, &plain, 0.0, 0),
            layout.visible_row_rects(bounds, &annotated, 0.0, 0)
        );
    }

    #[test]
    fn property_grid_status_presentation_is_deterministic() {
        assert_eq!(
            PropertyGridStatusSeverity::None.presentation().label,
            "None"
        );
        assert!(!PropertyGridStatusSeverity::None.presentation().accented);
        assert_eq!(
            PropertyGridStatusSeverity::Info.presentation().label,
            "Info"
        );
        assert!(PropertyGridStatusSeverity::Warning.presentation().accented);
        assert!(!PropertyGridStatusSeverity::Warning.presentation().blocking);
        assert!(PropertyGridStatusSeverity::Error.presentation().blocking);
        assert_eq!(
            PropertyGridRowStatus::error("Invalid").presentation(),
            PropertyGridStatusSeverity::Error.presentation()
        );
    }

    #[test]
    fn property_grid_status_semantics_include_severity_and_message_without_layout_changes() {
        let rows = [
            PropertyGridRow::property(ItemId::from_raw(1), "Mode", 0),
            PropertyGridRow::property(ItemId::from_raw(2), "Guide", 0)
                .with_status(PropertyGridRowStatus::info("Inherited from parent")),
            PropertyGridRow::property(ItemId::from_raw(3), "Exposure", 0)
                .with_status(PropertyGridRowStatus::warning("Preview range exceeded")),
            PropertyGridRow::property(ItemId::from_raw(4), "Mass", 0)
                .with_status(PropertyGridRowStatus::error("Mass must be positive")),
        ];
        let layout = PropertyGridLayout::new(20.0, 24.0, 90.0, 8.0, 12.0);
        let bounds = Rect::new(0.0, 0.0, 240.0, 80.0);
        let rects = layout.visible_row_rects(bounds, &rows, 0.0, 0);
        let plain_rows = [
            PropertyGridRow::property(ItemId::from_raw(1), "Mode", 0),
            PropertyGridRow::property(ItemId::from_raw(2), "Guide", 0),
            PropertyGridRow::property(ItemId::from_raw(3), "Exposure", 0),
            PropertyGridRow::property(ItemId::from_raw(4), "Mass", 0),
        ];

        assert_eq!(rects, layout.visible_row_rects(bounds, &plain_rows, 0.0, 0));
        assert!(
            property_grid_row_status_semantics(WidgetId::from_key("mode"), &rows[0], rects[0])
                .is_none()
        );

        for (index, expected) in [
            (1, "Info: Inherited from parent"),
            (2, "Warning: Preview range exceeded"),
            (3, "Error: Mass must be positive"),
        ] {
            let node = property_grid_row_status_semantics(
                WidgetId::from_key(rows[index].label.as_str()),
                &rows[index],
                rects[index],
            )
            .expect("status semantics");
            let expected_label = format!("{} status", rows[index].label);
            assert_eq!(node.role, SemanticRole::Label);
            assert_eq!(node.label.as_deref(), Some(expected_label.as_str()));
            assert_eq!(node.description.as_deref(), Some(expected));
            assert_eq!(
                node.state.value,
                Some(SemanticValue::Text(expected.to_owned()))
            );
        }
    }

    #[test]
    fn property_grid_affordance_rects_reserve_controls_without_changing_row_rect() {
        let row = PropertyGridRow::property(ItemId::from_raw(2), "Exposure", 0)
            .with_status(PropertyGridRowStatus::error("Too bright"))
            .with_resettable(true, false)
            .with_keyframeable(true, true);
        let layout = PropertyGridLayout::new(20.0, 24.0, 90.0, 8.0, 12.0);
        let row_rects = layout.visible_row_rects(
            Rect::new(0.0, 0.0, 220.0, 20.0),
            std::slice::from_ref(&row),
            0.0,
            0,
        );
        let row_rect = row_rects[0];
        let row_rect_without_status = layout.visible_row_rects(
            Rect::new(0.0, 0.0, 220.0, 20.0),
            &[
                PropertyGridRow::property(ItemId::from_raw(2), "Exposure", 0)
                    .with_resettable(true, false)
                    .with_keyframeable(true, true),
            ],
            0.0,
            0,
        )[0];

        assert_eq!(row_rect, row_rect_without_status);

        let affordances = property_grid_row_affordance_rects(
            &row,
            row_rect.value_rect,
            PropertyGridAffordanceLayout::new(18.0, 4.0),
        );
        assert!(affordances.reset_rect.is_some());
        assert!(affordances.keyframe_rect.is_some());
        assert!(affordances.value_rect.width < row_rect.value_rect.width);
        assert!(affordances.value_rect.max_x() <= affordances.reset_rect.unwrap().x);
    }

    #[test]
    fn property_grid_affordance_controls_emit_requests_only() {
        let theme = default_dark_theme();
        let row = PropertyGridRow::property(ItemId::from_raw(2), "Exposure", 0)
            .with_resettable(true, false)
            .with_keyframeable(true, false);
        let rects = property_grid_row_affordance_rects(
            &row,
            Rect::new(0.0, 0.0, 88.0, 20.0),
            PropertyGridAffordanceLayout::new(18.0, 4.0),
        );
        let reset_center = rects.reset_rect.expect("reset rect").center();
        let mut memory = UiMemory::new();

        let _ = property_grid_row_affordance_controls(
            WidgetId::from_key("exposure"),
            &row,
            rects,
            &pointer_input(reset_center.x, reset_center.y, true, true, false),
            &mut memory,
            &theme,
        );
        let reset = property_grid_row_affordance_controls(
            WidgetId::from_key("exposure"),
            &row,
            rects,
            &pointer_input(reset_center.x, reset_center.y, false, false, true),
            &mut memory,
            &theme,
        );

        assert!(reset.reset_requested);
        assert!(!reset.keyframe_toggle_requested);
        assert!(!reset.requested_keyed);

        let keyframe_center = rects.keyframe_rect.expect("keyframe rect").center();
        let mut memory = UiMemory::new();
        let _ = property_grid_row_affordance_controls(
            WidgetId::from_key("exposure"),
            &row,
            rects,
            &pointer_input(keyframe_center.x, keyframe_center.y, true, true, false),
            &mut memory,
            &theme,
        );
        let keyframe = property_grid_row_affordance_controls(
            WidgetId::from_key("exposure"),
            &row,
            rects,
            &pointer_input(keyframe_center.x, keyframe_center.y, false, false, true),
            &mut memory,
            &theme,
        );

        assert!(!keyframe.reset_requested);
        assert!(keyframe.keyframe_toggle_requested);
        assert!(keyframe.requested_keyed);
        assert!(!row.state.affordances.keyframe.keyed);
    }

    #[test]
    fn property_grid_affordance_controls_suppress_disabled_and_read_only_requests() {
        let theme = default_dark_theme();
        for row in [
            PropertyGridRow::property(ItemId::from_raw(2), "Exposure", 0)
                .with_disabled(true)
                .with_resettable(true, false)
                .with_keyframeable(true, false),
            PropertyGridRow::property(ItemId::from_raw(3), "Mass", 0)
                .with_read_only(true)
                .with_resettable(true, false)
                .with_keyframeable(true, false),
            PropertyGridRow::property(ItemId::from_raw(4), "Scale", 0)
                .with_resettable(true, true)
                .with_keyframeable(true, false),
        ] {
            let rects = property_grid_row_affordance_rects(
                &row,
                Rect::new(0.0, 0.0, 88.0, 20.0),
                PropertyGridAffordanceLayout::new(18.0, 4.0),
            );
            let reset_center = rects.reset_rect.expect("reset rect").center();
            let output = property_grid_row_affordance_controls(
                WidgetId::from_key(row.label.as_str()),
                &row,
                rects,
                &pointer_input(reset_center.x, reset_center.y, true, true, false),
                &mut UiMemory::new(),
                &theme,
            );

            assert!(!output.reset_requested);
            assert!(!output.keyframe_toggle_requested);
            assert!(
                output
                    .reset_response
                    .expect("reset response")
                    .state
                    .disabled
            );
            if row.state.disabled || row.state.read_only {
                assert!(
                    output
                        .keyframe_response
                        .expect("keyframe response")
                        .state
                        .disabled
                );
            }
        }
    }

    #[test]
    fn property_grid_affordance_controls_expose_semantics() {
        let theme = default_dark_theme();
        let row = PropertyGridRow::property(ItemId::from_raw(2), "Exposure", 0)
            .with_resettable(true, false)
            .with_keyframeable(true, true);
        let rects = property_grid_row_affordance_rects(
            &row,
            Rect::new(0.0, 0.0, 88.0, 20.0),
            PropertyGridAffordanceLayout::new(18.0, 4.0),
        );

        let output = property_grid_row_affordance_controls(
            WidgetId::from_key("exposure"),
            &row,
            rects,
            &UiInput::default(),
            &mut UiMemory::new(),
            &theme,
        );

        assert_eq!(output.widget.semantics.len(), 2);
        assert!(output.widget.semantics.iter().all(|node| {
            node.role == SemanticRole::IconButton
                && node
                    .actions
                    .iter()
                    .any(|action| action.kind == SemanticActionKind::Invoke)
        }));
        assert!(output.widget.semantics.iter().any(|node| {
            node.label.as_deref() == Some("Reset Exposure to default") && !node.state.selected
        }));
        assert!(output.widget.semantics.iter().any(|node| {
            node.label.as_deref() == Some("Toggle keyframe for Exposure") && node.state.selected
        }));
    }

    #[test]
    fn property_grid_sanitizes_invalid_sizes() {
        let rows = rows();
        let layout = PropertyGridLayout::new(f32::NAN, -1.0, f32::NAN, f32::NAN, -12.0);

        assert_approx(layout.content_height(&rows), 0.0);
        assert_eq!(layout.visible_range(&rows, 0.0, 44.0, 0), 0..0);
        let rects = layout.visible_row_rects(Rect::new(10.0, 20.0, 100.0, 44.0), &rows, 0.0, 0);
        assert!(rects.is_empty());
    }

    #[test]
    fn vector_component_rects_split_vec2_vec3_and_vec4_without_overlap() {
        let layout = VectorComponentLayout::new(6.0, 10.0, 3.0, 24.0);
        let bounds = Rect::new(10.0, 20.0, 300.0, 24.0);

        let vec2 = vector2_component_rects(bounds, layout);
        assert_eq!(vec2.len(), 2);
        assert_eq!(vec2[0].label, "X");
        assert_eq!(vec2[1].label, "Y");
        assert_approx(vec2[0].rect.width, 147.0);
        assert_approx(vec2[1].rect.x, 163.0);

        let vec3 = vector3_component_rects(bounds, layout);
        assert_eq!(vec3.len(), 3);
        assert_eq!(vec3[2].label, "Z");
        assert!(vec3[0].rect.max_x() <= vec3[1].rect.x);
        assert!(vec3[1].rect.max_x() <= vec3[2].rect.x);

        let vec4 = vector4_component_rects(bounds, layout);
        assert_eq!(vec4.len(), 4);
        assert_eq!(vec4[3].label, "W");
        for component in vec4 {
            assert!(component.label_rect.max_x() <= component.value_rect.x);
            assert!(component.value_rect.max_x() <= component.rect.max_x());
        }
    }

    #[test]
    fn vector_component_rects_clamp_narrow_and_invalid_widths() {
        let layout = VectorComponentLayout::new(f32::NAN, 12.0, f32::INFINITY, 40.0);
        let narrow = vector3_component_rects(Rect::new(0.0, 0.0, 42.0, 18.0), layout);

        assert_approx(narrow[0].rect.width, 14.0);
        assert_vector_components_finite_and_non_overlapping(&narrow);
        for component in narrow {
            assert!(component.label_rect.width <= component.rect.width);
            assert!(component.value_rect.width >= 0.0);
            assert!(component.value_rect.max_x() <= component.rect.max_x());
        }

        let invalid = vector4_component_rects(
            Rect::new(0.0, 0.0, f32::NAN, 18.0),
            VectorComponentLayout::default(),
        );
        assert!(
            invalid.iter().all(|component| {
                component.rect.width == 0.0 && component.value_rect.width == 0.0
            })
        );
    }

    #[test]
    fn vector_component_rects_sanitize_invalid_gaps_for_placement() {
        let bounds = Rect::new(10.0, 20.0, 120.0, 24.0);
        let invalid_gaps = [f32::NAN, f32::INFINITY, -8.0];

        for component_gap in invalid_gaps {
            let components = vector4_component_rects(
                bounds,
                VectorComponentLayout::new(component_gap, 10.0, 3.0, 24.0),
            );
            assert_vector_components_finite_and_non_overlapping(&components);
        }
    }
}
