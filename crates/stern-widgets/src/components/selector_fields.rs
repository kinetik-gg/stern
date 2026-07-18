use super::{
    ComponentState, CursorShape, DropTargetResponse, DropdownModel, DropdownTriggerPresentation,
    Primitive, Rect, RectPrimitive, Response, SemanticAction, SemanticActionKind, SemanticNode,
    SemanticRole, SemanticValue, StaticIcon, TextEditState, TextFieldAccess, TextFieldOutput,
    TextLayoutKey, TextLayoutStore, TextOverflow, TextStyle, Theme, UiInput, UiMemory, WidgetId,
    WidgetOutput, drop_target, field_text_primitive, finite_widget_extent, pressable,
    suppress_disabled_interaction_reporting, text_field_with_access_runtime_metadata_and_fence,
    text_field_with_text_layouts_and_caret_visibility, with_hover_cursor, with_response_state,
};
use stern_core::Ui as CoreUi;

/// Configuration for an inspector select/enum field backed by a dropdown model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectFieldConfig {
    /// Placeholder shown when no enabled item is selected.
    pub placeholder: String,
    /// Whether the dropdown overlay for this select is currently open.
    pub open: bool,
    /// Whether the field is disabled.
    pub disabled: bool,
    /// Whether the field is displayed but cannot be changed.
    pub read_only: bool,
}

impl SelectFieldConfig {
    /// Creates a select field configuration.
    #[must_use]
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: placeholder.into(),
            open: false,
            disabled: false,
            read_only: false,
        }
    }

    /// Sets whether the backing dropdown overlay is open.
    #[must_use]
    pub const fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Sets whether the field is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets whether the field is read-only.
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }
}

impl Default for SelectFieldConfig {
    fn default() -> Self {
        Self::new("Select...")
    }
}

/// Output emitted by an inspector select/enum field.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectFieldOutput {
    /// Base widget output.
    pub widget: WidgetOutput,
    /// Press/open response for the trigger.
    pub response: Response,
    /// Resolved closed-trigger presentation metadata.
    pub presentation: DropdownTriggerPresentation,
    /// Whether the field requested that the application open its dropdown.
    pub open_requested: bool,
    /// Whether the field is read-only.
    pub read_only: bool,
}

/// Metadata for a generic asset assigned to an asset slot field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSlotAsset {
    /// Stable application-owned asset identity.
    pub identity: String,
    /// User-visible asset label.
    pub label: String,
    /// Optional application-owned asset kind label.
    pub kind: Option<String>,
    /// Optional static icon metadata for renderers or host applications.
    pub icon: Option<StaticIcon>,
}

impl AssetSlotAsset {
    /// Creates asset metadata for a filled slot.
    #[must_use]
    pub fn new(identity: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            identity: identity.into(),
            label: label.into(),
            kind: None,
            icon: None,
        }
    }

    /// Returns this metadata with an asset kind label.
    #[must_use]
    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Returns this metadata with a static icon.
    #[must_use]
    pub fn with_icon(mut self, icon: impl Into<StaticIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// Configuration for an inspector asset slot field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSlotConfig {
    /// Label shown when the slot is empty.
    pub empty_label: String,
    /// Whether the field is disabled.
    pub disabled: bool,
    /// Whether the field is displayed but cannot be changed.
    pub read_only: bool,
    /// Whether the slot should resolve drop-target behavior.
    pub accepts_drop: bool,
}

impl AssetSlotConfig {
    /// Creates an asset slot configuration.
    #[must_use]
    pub fn new(empty_label: impl Into<String>) -> Self {
        Self {
            empty_label: empty_label.into(),
            disabled: false,
            read_only: false,
            accepts_drop: false,
        }
    }

    /// Sets whether the field is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets whether the field is read-only.
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Sets whether the field resolves drop-target behavior.
    #[must_use]
    pub const fn accepts_drop(mut self, accepts_drop: bool) -> Self {
        self.accepts_drop = accepts_drop;
        self
    }
}

impl Default for AssetSlotConfig {
    fn default() -> Self {
        Self::new("None")
    }
}

/// Output emitted by an inspector asset slot field.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct AssetSlotOutput {
    /// Base widget output.
    pub widget: WidgetOutput,
    /// Press response for the slot.
    pub response: Response,
    /// Optional drop-target response when drop behavior was requested.
    pub drop_target: Option<DropTargetResponse>,
    /// Whether the slot contains asset metadata.
    pub filled: bool,
    /// Whether the field requested that the application show an asset picker.
    pub pick_requested: bool,
    /// Whether the field requested that the application open the current asset.
    pub open_requested: bool,
    /// Whether an eligible drag source was released over the slot.
    pub drop_received: bool,
    /// Whether the field is read-only.
    pub read_only: bool,
}

/// Configuration for an inspector file/path text field.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct PathFieldConfig {
    /// Width of the trailing browse button.
    pub browse_width: f32,
    /// Gap between text input and browse button.
    pub gap: f32,
    /// Whether the field is disabled.
    pub disabled: bool,
    /// Whether the path is displayed but cannot be edited or browsed.
    pub read_only: bool,
    /// Whether the browse affordance should be presented.
    pub browse: bool,
    /// Whether double-clicking the text field may request app-owned open behavior.
    pub open: bool,
}

impl PathFieldConfig {
    /// Creates a path field configuration.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            browse_width: 28.0,
            gap: 4.0,
            disabled: false,
            read_only: false,
            browse: true,
            open: false,
        }
    }

    /// Sets the browse button width.
    #[must_use]
    pub const fn with_browse_width(mut self, browse_width: f32) -> Self {
        self.browse_width = browse_width;
        self
    }

    /// Sets the gap between text input and browse button.
    #[must_use]
    pub const fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Sets whether the field is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets whether the field is read-only.
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Sets whether the browse affordance is present.
    #[must_use]
    pub const fn browse(mut self, browse: bool) -> Self {
        self.browse = browse;
        self
    }

    /// Sets whether double-clicking the text field may request an app-owned open action.
    #[must_use]
    pub const fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

impl Default for PathFieldConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Output emitted by an inspector path field.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct PathFieldOutput {
    /// Aggregated widget output for the text field and optional browse button.
    pub widget: WidgetOutput,
    /// Text field output for editing the path string.
    pub field: TextFieldOutput,
    /// Optional browse button response.
    pub browse_response: Option<Response>,
    /// Whether text changed this frame.
    pub changed: bool,
    /// Whether the field requested app-owned browse behavior.
    pub browse_requested: bool,
    /// Whether the field requested app-owned open behavior.
    pub open_requested: bool,
    /// Whether the field is read-only.
    pub read_only: bool,
}

/// Emits an inspector select/enum field backed by a dropdown model.
#[allow(clippy::too_many_arguments)]
pub fn select_field(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    model: &DropdownModel,
    config: SelectFieldConfig,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
) -> SelectFieldOutput {
    select_field_with_text_layouts(id, rect, label, model, config, input, memory, theme, None)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn select_field_with_text_layouts(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    model: &DropdownModel,
    config: SelectFieldConfig,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    text_layouts: Option<&mut TextLayoutStore>,
) -> SelectFieldOutput {
    let label = label.into();
    let has_enabled_item = model.items().iter().any(|item| item.enabled);
    let interactions_disabled = config.disabled || config.read_only || !has_enabled_item;
    let mut response = pressable(id, rect, input, memory, interactions_disabled);
    suppress_disabled_interaction_reporting(&mut response);
    let presentation =
        model.trigger_presentation(config.placeholder, interactions_disabled, config.open);
    let recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled: interactions_disabled,
        selected: presentation.selected(),
    });
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius: recipe.radius,
    })];
    let arrow_width = 16.0_f32.min(rect.width.max(0.0));
    let text_rect = Rect::new(
        rect.x + recipe.padding_x,
        rect.y,
        (rect.width - recipe.padding_x * 2.0 - arrow_width).max(0.0),
        rect.height,
    );
    let mut value_primitive =
        field_text_primitive(text_rect, presentation.label.clone(), &recipe, theme);
    if let (Some(text_layouts), Primitive::Text(text)) = (text_layouts, &mut value_primitive) {
        text.layout = text_layouts.try_layout_id(
            TextLayoutKey::new(
                text.text.clone(),
                TextStyle::new(text.family.clone(), text.size, text.line_height),
                text_rect.width,
                false,
            )
            .with_overflow(TextOverflow::EndEllipsis),
        );
    }
    primitives.push(value_primitive);
    if arrow_width > 0.0 {
        primitives.push(field_text_primitive(
            Rect::new(rect.max_x() - arrow_width, rect.y, arrow_width, rect.height),
            if presentation.open { "^" } else { "v" },
            &recipe,
            theme,
        ));
    }

    let mut node = SemanticNode::new(id, SemanticRole::Button, rect)
        .with_label(label)
        .focusable(!interactions_disabled);
    node.description = Some(presentation.label.clone());
    node.state.disabled = interactions_disabled;
    node.state.selected = presentation.selected();
    node.state.expanded = Some(presentation.open);
    node.state.value = Some(SemanticValue::Text(presentation.label.clone()));
    if !interactions_disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Open,
            "Open choices",
        ));
    }

    let output = WidgetOutput::new(Some(response), primitives).with_semantic(node);
    let output = with_hover_cursor(output, &response, CursorShape::PointingHand);

    SelectFieldOutput {
        widget: output,
        response,
        presentation,
        open_requested: !interactions_disabled && (response.clicked || response.keyboard_activated),
        read_only: config.read_only,
    }
}

/// Emits an inspector asset slot field.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub fn asset_slot_field(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    asset: Option<&AssetSlotAsset>,
    config: AssetSlotConfig,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
) -> AssetSlotOutput {
    let label = label.into();
    let interactions_disabled = config.disabled || config.read_only;
    let drop_response = config
        .accepts_drop
        .then(|| drop_target(id.child("drop"), rect, input, memory, interactions_disabled));
    let drop_hovered = drop_response
        .as_ref()
        .is_some_and(|drop| drop.response.state.hovered);
    let drop_received = drop_response.as_ref().is_some_and(|drop| drop.dropped);
    let mut response = pressable(id, rect, input, memory, interactions_disabled);
    suppress_disabled_interaction_reporting(&mut response);
    let filled = asset.is_some();
    let recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered || drop_hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled: interactions_disabled,
        selected: drop_hovered,
    });
    let value_label = asset.map_or_else(|| config.empty_label.clone(), |asset| asset.label.clone());
    let detail_label = asset
        .and_then(|asset| asset.kind.as_deref())
        .map_or_else(String::new, |kind| format!(" ({kind})"));
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius: recipe.radius,
    })];
    let text_rect = Rect::new(
        rect.x + recipe.padding_x,
        rect.y,
        (rect.width - recipe.padding_x * 2.0).max(0.0),
        rect.height,
    );
    primitives.push(field_text_primitive(
        text_rect,
        format!("{value_label}{detail_label}"),
        &recipe,
        theme,
    ));

    let mut node = SemanticNode::new(id, SemanticRole::Button, rect)
        .with_label(label)
        .focusable(!interactions_disabled);
    node.description = asset
        .and_then(|asset| asset.kind.as_ref())
        .cloned()
        .or_else(|| Some("Empty asset slot".to_owned()));
    node.state.disabled = interactions_disabled;
    node.state.selected = filled;
    node.state.value = Some(SemanticValue::Text(value_label));
    if !interactions_disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Invoke,
            "Choose asset",
        ));
        if filled {
            node.actions
                .push(SemanticAction::new(SemanticActionKind::Open, "Open asset"));
        }
    }

    let output = WidgetOutput::new(Some(response), primitives).with_semantic(node);
    let output = with_hover_cursor(output, &response, CursorShape::PointingHand);
    let open_requested =
        !interactions_disabled && filled && (response.double_clicked || response.secondary_clicked);

    AssetSlotOutput {
        widget: output,
        response,
        drop_target: drop_response,
        filled,
        pick_requested: !interactions_disabled
            && !open_requested
            && (response.clicked || response.keyboard_activated),
        open_requested,
        drop_received,
        read_only: config.read_only,
    }
}

/// Emits an inspector path field.
#[allow(clippy::too_many_arguments)]
pub fn path_field(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    state: &mut TextEditState,
    config: PathFieldConfig,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
) -> PathFieldOutput {
    path_field_with_text_layouts(id, rect, label, state, config, input, memory, theme, None)
}

/// Emits an inspector path field using shaped text layout cache when available.
#[allow(clippy::too_many_arguments)]
pub fn path_field_with_text_layouts(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    state: &mut TextEditState,
    config: PathFieldConfig,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    text_layouts: Option<&mut TextLayoutStore>,
) -> PathFieldOutput {
    path_field_with_text_layouts_and_caret_visibility(
        id,
        rect,
        label,
        state,
        config,
        input,
        memory,
        theme,
        text_layouts,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn path_field_with_text_layouts_and_caret_visibility(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    state: &mut TextEditState,
    config: PathFieldConfig,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> PathFieldOutput {
    let label = label.into();
    let interactions_disabled = config.disabled || config.read_only;
    let browse_width = if config.browse {
        finite_widget_extent(config.browse_width).min(rect.width.max(0.0))
    } else {
        0.0
    };
    let gap = if browse_width > 0.0 {
        finite_widget_extent(config.gap).min((rect.width - browse_width).max(0.0))
    } else {
        0.0
    };
    let field_width = (rect.width - browse_width - gap).max(0.0);
    let field_rect = Rect::new(rect.x, rect.y, field_width, rect.height);
    let button_rect = Rect::new(field_rect.max_x() + gap, rect.y, browse_width, rect.height);
    let mut field = text_field_with_text_layouts_and_caret_visibility(
        id.child("text"),
        field_rect,
        state,
        input,
        memory,
        theme,
        interactions_disabled,
        text_layouts,
        caret_visible,
    );
    if let Some(node) = field.widget.semantics.first_mut() {
        node.label = Some(label.clone());
    }

    let mut browse_response = None;
    let mut browse_requested = false;
    let mut widget = field.widget.clone();
    if browse_width > 0.0 {
        let browse_id = id.child("browse");
        let mut response = pressable(browse_id, button_rect, input, memory, interactions_disabled);
        suppress_disabled_interaction_reporting(&mut response);
        let recipe = theme.text_field(ComponentState {
            hovered: response.state.hovered,
            pressed: response.state.pressed,
            focused: response.state.focused,
            disabled: interactions_disabled,
            selected: false,
        });
        widget.primitives.push(Primitive::Rect(RectPrimitive {
            rect: button_rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
        widget.primitives.push(field_text_primitive(
            button_rect.inset(2.0),
            "...",
            &recipe,
            theme,
        ));
        let mut node = SemanticNode::new(browse_id, SemanticRole::Button, button_rect)
            .with_label(format!("Browse {label}"))
            .focusable(!interactions_disabled);
        node.state.disabled = interactions_disabled;
        if !interactions_disabled {
            node.actions
                .push(SemanticAction::new(SemanticActionKind::Open, "Browse"));
        }
        widget.semantics.push(with_response_state(node, &response));
        browse_requested =
            !interactions_disabled && (response.clicked || response.keyboard_activated);
        browse_response = Some(response);
    }

    let open_requested = config.open
        && !interactions_disabled
        && !state.text.is_empty()
        && field
            .widget
            .response
            .as_ref()
            .is_some_and(|response| response.double_clicked);

    PathFieldOutput {
        widget,
        changed: field.changed,
        field,
        browse_response,
        browse_requested,
        open_requested,
        read_only: config.read_only,
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) fn path_field_with_access_runtime(
    runtime: &mut CoreUi<'_>,
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    state: &mut TextEditState,
    config: PathFieldConfig,
    theme: &Theme,
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> PathFieldOutput {
    let label = label.into();
    let access = if config.disabled {
        TextFieldAccess::Disabled
    } else if config.read_only {
        TextFieldAccess::ReadOnly
    } else {
        TextFieldAccess::Editable
    };
    let browse_disabled = config.disabled || config.read_only;
    let browse_width = if config.browse {
        finite_widget_extent(config.browse_width).min(rect.width.max(0.0))
    } else {
        0.0
    };
    let gap = if browse_width > 0.0 {
        finite_widget_extent(config.gap).min((rect.width - browse_width).max(0.0))
    } else {
        0.0
    };
    let field_width = (rect.width - browse_width - gap).max(0.0);
    let field_rect = Rect::new(rect.x, rect.y, field_width, rect.height);
    let button_rect = Rect::new(field_rect.max_x() + gap, rect.y, browse_width, rect.height);
    let browse_response = if browse_width > 0.0 {
        let browse_id = id.child("browse");
        let mut response = {
            let (input, memory) = runtime.input_and_memory_mut();
            pressable(browse_id, button_rect, input, memory, browse_disabled)
        };
        suppress_disabled_interaction_reporting(&mut response);
        Some(response)
    } else {
        None
    };
    let browse_requested = browse_response.as_ref().is_some_and(|response| {
        !browse_disabled && (response.clicked || response.keyboard_activated)
    });
    let (mut field, _, pointer) = text_field_with_access_runtime_metadata_and_fence(
        runtime,
        id.child("text"),
        field_rect,
        state,
        theme,
        access,
        text_layouts,
        caret_visible,
        browse_requested,
    );
    if let Some(node) = field.widget.semantics.first_mut() {
        node.label = Some(label.clone());
    }

    let mut widget = field.widget.clone();
    if let Some(response) = browse_response.as_ref() {
        let browse_id = id.child("browse");
        let recipe = theme.text_field(ComponentState {
            hovered: response.state.hovered,
            pressed: response.state.pressed,
            focused: response.state.focused,
            disabled: browse_disabled,
            selected: false,
        });
        widget.primitives.push(Primitive::Rect(RectPrimitive {
            rect: button_rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
        widget.primitives.push(field_text_primitive(
            button_rect.inset(2.0),
            "...",
            &recipe,
            theme,
        ));
        let mut node = SemanticNode::new(browse_id, SemanticRole::Button, button_rect)
            .with_label(format!("Browse {label}"))
            .focusable(!browse_disabled);
        node.state.disabled = browse_disabled;
        if !browse_disabled {
            node.actions
                .push(SemanticAction::new(SemanticActionKind::Open, "Browse"));
        }
        widget.semantics.push(with_response_state(node, response));
    }

    let open_requested = config.open
        && !browse_disabled
        && !browse_requested
        && !state.text.is_empty()
        && pointer.accepted_double_click;
    PathFieldOutput {
        widget,
        changed: field.changed,
        field,
        browse_response,
        browse_requested,
        open_requested,
        read_only: config.read_only,
    }
}
