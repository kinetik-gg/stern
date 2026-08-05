//! Content-sized widget builders and the layout-tree seam (RFC 0001 Phase L1).
//!
//! This module is the seam between the L0 layout solver in `stern-core` and
//! the existing rect-first widget methods on [`Ui`]. Builders measure their
//! own content — shaped text through the frame's `TextLayoutStore`, control
//! chrome through theme metrics — and compose by delegating to the existing
//! `Ui` methods, which stay untouched. Nothing about the rect-first surface
//! changes; `ui.button(key, rect, …)` and `ui.layout(bounds, …)` coexist.
//!
//! The seam honors the closed-world pointer plan: [`Ui::layout`] returns
//! solved, screen-space geometry *before* any widget behavior runs, so
//! callers can declare pointer targets from [`UiLayout::rect`] and then
//! compose.

use std::hash::Hash;

use stern_core::{
    Alignment, FixedMeasure, Insets, LayoutNodeId, LayoutTree, Measurement, Rect, Response, Size,
    SizeRule, StaticIcon, TextRole, Theme,
};
use stern_text::{TextLayoutKey, TextLayoutStore, TextStyle};

use super::Ui;

/// Measurement context handed to [`Widget::measure`].
///
/// Wraps the theme and, when the frame has one, the shaped-text store. Text
/// measurement without a store deterministically reports zero extent; frames
/// that want content-sized text must run with text layouts enabled.
pub struct MeasureContext<'m> {
    theme: &'m Theme,
    text_layouts: Option<&'m mut TextLayoutStore>,
}

impl<'m> MeasureContext<'m> {
    /// Creates a measurement context.
    #[must_use]
    pub fn new(theme: &'m Theme, text_layouts: Option<&'m mut TextLayoutStore>) -> Self {
        Self {
            theme,
            text_layouts,
        }
    }

    /// Returns the theme measurements resolve against.
    #[must_use]
    pub fn theme(&self) -> &Theme {
        self.theme
    }

    /// Measures a single line of text in the given role, unconstrained.
    ///
    /// Returns [`Size::ZERO`] when the frame has no text-layout store.
    pub fn measure_text(&mut self, text: &str, role: TextRole) -> Size {
        let font = self.theme.font(role);
        let style = TextStyle::new(font.family.to_owned(), font.size, font.line_height);
        self.measure_styled_text(text, style)
    }

    fn measure_styled_text(&mut self, text: &str, style: TextStyle) -> Size {
        let Some(store) = self.text_layouts.as_deref_mut() else {
            return Size::ZERO;
        };
        store
            .shape_transient(&TextLayoutKey::new(text, style, f32::MAX, false))
            .size
    }
}

/// A widget that can measure its own content and compose into a solved rect.
///
/// This is the RFC 0001 §8 builder trait (accepted default): object-safe so
/// the layout scope can store heterogeneous leaves, with composition
/// delegating to the existing rect-first `Ui` methods.
pub trait Widget {
    /// Returns the intrinsic measurement of this widget's content.
    fn measure(&self, ctx: &mut MeasureContext<'_>) -> Measurement;

    /// Returns the default size rules used when added without overrides.
    fn size_rules(&self) -> (SizeRule, SizeRule) {
        (SizeRule::Fit, SizeRule::Fit)
    }

    /// Composes the widget into its solved rectangle.
    ///
    /// Passive widgets return `None`.
    fn compose(self: Box<Self>, ui: &mut Ui<'_>, rect: Rect) -> Option<Response>;
}

/// Content-sized push button builder.
///
/// Measured width is `2 × theme.controls.padding_x` plus the shaped label
/// width (RFC 0001 §7.2); measured height is `theme.controls.control_height`.
pub struct Button {
    key: String,
    label: String,
    disabled: bool,
}

impl Button {
    /// Creates a button builder.
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            disabled: false,
        }
    }

    /// Sets whether the button is disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Widget for Button {
    fn measure(&self, ctx: &mut MeasureContext<'_>) -> Measurement {
        let text = ctx.measure_text(&self.label, TextRole::Label);
        let controls = ctx.theme().controls;
        Measurement::new(Size::new(
            controls.padding_x * 2.0 + text.width,
            controls.control_height,
        ))
    }

    fn compose(self: Box<Self>, ui: &mut Ui<'_>, rect: Rect) -> Option<Response> {
        Some(ui.button(self.key, rect, self.label, self.disabled))
    }
}

/// Passive text label builder measured from its shaped body text.
pub struct Label {
    text: String,
}

impl Label {
    /// Creates a label builder.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for Label {
    fn measure(&self, ctx: &mut MeasureContext<'_>) -> Measurement {
        let font = ctx.theme().label(TextRole::Body, false).font;
        let style = TextStyle::new(font.family.to_owned(), font.size, font.line_height);
        Measurement::new(ctx.measure_styled_text(&self.text, style))
    }

    fn compose(self: Box<Self>, ui: &mut Ui<'_>, rect: Rect) -> Option<Response> {
        ui.label(rect, self.text);
        None
    }
}

/// Content-sized icon button builder: a control-height square.
pub struct IconButton {
    key: String,
    icon: StaticIcon,
    label: String,
    disabled: bool,
}

impl IconButton {
    /// Creates an icon button builder with a required accessible label.
    pub fn new(
        key: impl Into<String>,
        icon: impl Into<StaticIcon>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            icon: icon.into(),
            label: label.into(),
            disabled: false,
        }
    }

    /// Sets whether the icon button is disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Widget for IconButton {
    fn measure(&self, ctx: &mut MeasureContext<'_>) -> Measurement {
        let side = ctx.theme().controls.control_height;
        Measurement::new(Size::new(side, side))
    }

    fn compose(self: Box<Self>, ui: &mut Ui<'_>, rect: Rect) -> Option<Response> {
        Some(ui.icon_button(self.key, rect, self.icon, self.label, self.disabled))
    }
}

/// Content-sized checkbox builder: the theme's check box square.
pub struct Checkbox {
    key: String,
    label: String,
    checked: bool,
    disabled: bool,
}

impl Checkbox {
    /// Creates a checkbox builder with an accessible label.
    pub fn new(key: impl Into<String>, label: impl Into<String>, checked: bool) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            checked,
            disabled: false,
        }
    }

    /// Sets whether the checkbox is disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Widget for Checkbox {
    fn measure(&self, ctx: &mut MeasureContext<'_>) -> Measurement {
        let side = ctx
            .theme()
            .checkbox(stern_core::ComponentState::default())
            .size;
        Measurement::new(Size::new(side, side))
    }

    fn compose(self: Box<Self>, ui: &mut Ui<'_>, rect: Rect) -> Option<Response> {
        Some(ui.checkbox_with_label(self.key, rect, self.label, self.checked, self.disabled))
    }
}

/// Content-sized radio button builder: the theme's radio circle square.
pub struct RadioButton {
    key: String,
    label: String,
    selected: bool,
    disabled: bool,
}

impl RadioButton {
    /// Creates a radio button builder with an accessible label.
    pub fn new(key: impl Into<String>, label: impl Into<String>, selected: bool) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            selected,
            disabled: false,
        }
    }

    /// Sets whether the radio button is disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Widget for RadioButton {
    fn measure(&self, ctx: &mut MeasureContext<'_>) -> Measurement {
        let side = ctx
            .theme()
            .radio_button(stern_core::ComponentState::default())
            .size;
        Measurement::new(Size::new(side, side))
    }

    fn compose(self: Box<Self>, ui: &mut Ui<'_>, rect: Rect) -> Option<Response> {
        Some(ui.radio_button_with_label(self.key, rect, self.label, self.selected, self.disabled))
    }
}

/// Toggle (switch) track dimensions from `docs/visual-spec/03` ("Track:
/// 26×14"). The design system exposes no switch-size token yet; when one
/// lands this constant must be replaced by it.
const TOGGLE_TRACK: Size = Size::new(26.0, 14.0);

/// Content-sized toggle builder: the visual-spec switch track.
pub struct Toggle {
    key: String,
    label: String,
    on: bool,
    disabled: bool,
}

impl Toggle {
    /// Creates a toggle builder with an accessible label.
    pub fn new(key: impl Into<String>, label: impl Into<String>, on: bool) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            on,
            disabled: false,
        }
    }

    /// Sets whether the toggle is disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Widget for Toggle {
    fn measure(&self, _ctx: &mut MeasureContext<'_>) -> Measurement {
        Measurement::new(TOGGLE_TRACK)
    }

    fn compose(self: Box<Self>, ui: &mut Ui<'_>, rect: Rect) -> Option<Response> {
        Some(ui.toggle_with_label(self.key, rect, self.label, self.on, self.disabled))
    }
}

/// Slider builder.
///
/// Sliders have no intrinsic width; the default width rule is [`SizeRule::Fill`]
/// and the `Fit` fallback reports four control heights. Measured height is the
/// spec's slider row height (`theme.sizes.control.md`).
pub struct Slider<'v> {
    key: String,
    label: String,
    value: &'v mut f32,
    range: core::ops::RangeInclusive<f32>,
    disabled: bool,
}

impl<'v> Slider<'v> {
    /// Creates a slider builder with an accessible label.
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        value: &'v mut f32,
        range: core::ops::RangeInclusive<f32>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value,
            range,
            disabled: false,
        }
    }

    /// Sets whether the slider is disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Widget for Slider<'_> {
    fn measure(&self, ctx: &mut MeasureContext<'_>) -> Measurement {
        let height = ctx.theme().sizes.control.md;
        Measurement::new(Size::new(height * 4.0, height))
    }

    fn size_rules(&self) -> (SizeRule, SizeRule) {
        (SizeRule::Fill, SizeRule::Fit)
    }

    fn compose(self: Box<Self>, ui: &mut Ui<'_>, rect: Rect) -> Option<Response> {
        let this = *self;
        Some(ui.slider_with_label(
            this.key,
            rect,
            this.label,
            this.value,
            this.range,
            this.disabled,
        ))
    }
}

/// Handle to a widget or rect slot inside a solved [`UiLayout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutSlot(usize);

/// Tree-building scope passed to the [`Ui::layout`] closure.
///
/// Widgets are measured eagerly as they are added — the scope has the frame's
/// theme and text store — and composed later from solved rects, preserving
/// the geometry-before-behavior pipeline the pointer plan requires.
pub struct LayoutScope<'s, 'a, 'f> {
    ui: &'s mut Ui<'a>,
    tree: LayoutTree,
    widgets: Vec<Option<Box<dyn Widget + 'f>>>,
    slot_nodes: Vec<LayoutNodeId>,
    children_stack: Vec<Vec<LayoutNodeId>>,
}

impl<'s, 'a, 'f> LayoutScope<'s, 'a, 'f> {
    fn new(ui: &'s mut Ui<'a>) -> Self {
        Self {
            ui,
            tree: LayoutTree::new(),
            widgets: Vec::new(),
            slot_nodes: Vec::new(),
            children_stack: vec![Vec::new()],
        }
    }

    fn measure_widget(&mut self, widget: &impl Widget) -> Measurement {
        let mut ctx = MeasureContext::new(self.ui.theme, self.ui.text_layouts.as_deref_mut());
        widget.measure(&mut ctx)
    }

    fn push_slot(
        &mut self,
        node: LayoutNodeId,
        widget: Option<Box<dyn Widget + 'f>>,
    ) -> LayoutSlot {
        let slot = LayoutSlot(self.slot_nodes.len());
        self.slot_nodes.push(node);
        self.widgets.push(widget);
        self.children_stack
            .last_mut()
            .expect("layout scope stack is never empty")
            .push(node);
        slot
    }

    /// Adds a widget with its default size rules.
    pub fn add(&mut self, widget: impl Widget + 'f) -> LayoutSlot {
        let (width, height) = widget.size_rules();
        self.add_sized(widget, width, height)
    }

    /// Adds a widget with explicit size rules.
    pub fn add_sized(
        &mut self,
        widget: impl Widget + 'f,
        width: SizeRule,
        height: SizeRule,
    ) -> LayoutSlot {
        let measurement = self.measure_widget(&widget);
        let node = self
            .tree
            .leaf(width, height, FixedMeasure::new(measurement));
        self.push_slot(node, Some(Box::new(widget)))
    }

    /// Adds an empty rectangle slot (for panels and externally composed content).
    pub fn slot(&mut self, width: SizeRule, height: SizeRule) -> LayoutSlot {
        let node = self
            .tree
            .leaf(width, height, FixedMeasure::new(Measurement::default()));
        self.push_slot(node, None)
    }

    fn container(
        &mut self,
        build: impl FnOnce(&mut Self),
        finish: impl FnOnce(&mut LayoutTree, Vec<LayoutNodeId>) -> LayoutNodeId,
    ) -> LayoutSlot {
        self.children_stack.push(Vec::new());
        build(self);
        let children = self
            .children_stack
            .pop()
            .expect("layout scope stack is never empty");
        let node = finish(&mut self.tree, children);
        self.push_slot(node, None)
    }

    /// Adds a horizontal row container.
    pub fn row(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        spacing: f32,
        build: impl FnOnce(&mut Self),
    ) -> LayoutSlot {
        self.container(build, move |tree, children| {
            tree.row(width, height, spacing, children)
        })
    }

    /// Adds a vertical column container.
    pub fn column(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        spacing: f32,
        build: impl FnOnce(&mut Self),
    ) -> LayoutSlot {
        self.container(build, move |tree, children| {
            tree.column(width, height, spacing, children)
        })
    }

    /// Adds a stack container overlaying its children.
    pub fn stack(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        build: impl FnOnce(&mut Self),
    ) -> LayoutSlot {
        self.container(build, move |tree, children| {
            tree.stack(width, height, children)
        })
    }

    /// Adds a padding container around the closure's content.
    ///
    /// If the closure adds more than one child, the children are wrapped in a
    /// zero-spacing column.
    pub fn padding(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        insets: Insets,
        build: impl FnOnce(&mut Self),
    ) -> LayoutSlot {
        self.container(build, move |tree, children| {
            let child = single_child(tree, children);
            tree.padding(width, height, insets, child)
        })
    }

    /// Adds an alignment container around the closure's content.
    ///
    /// If the closure adds more than one child, the children are wrapped in a
    /// zero-spacing column.
    pub fn align(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        horizontal: Alignment,
        vertical: Alignment,
        build: impl FnOnce(&mut Self),
    ) -> LayoutSlot {
        self.container(build, move |tree, children| {
            let child = single_child(tree, children);
            tree.align(width, height, horizontal, vertical, child)
        })
    }
}

fn single_child(tree: &mut LayoutTree, mut children: Vec<LayoutNodeId>) -> LayoutNodeId {
    if children.len() == 1 {
        children.remove(0)
    } else {
        tree.column(SizeRule::Fit, SizeRule::Fit, 0.0, children)
    }
}

/// A solved layout whose geometry is final before any widget behavior runs.
///
/// Query [`Self::rect`] to declare pointer targets, then [`Self::compose`] to
/// emit the stored widgets into their solved rectangles.
pub struct UiLayout<'f> {
    solution: stern_core::LayoutSolution,
    slot_nodes: Vec<LayoutNodeId>,
    widgets: Vec<Option<Box<dyn Widget + 'f>>>,
}

impl UiLayout<'_> {
    /// Returns the solved rectangle for a slot.
    #[must_use]
    pub fn rect(&self, slot: LayoutSlot) -> Rect {
        self.solution.rect(self.slot_nodes[slot.0])
    }

    /// Returns the intrinsic measurement recorded for a slot.
    #[must_use]
    pub fn measurement(&self, slot: LayoutSlot) -> Measurement {
        self.solution.measurement(self.slot_nodes[slot.0])
    }

    /// Composes every stored widget into its solved rectangle.
    pub fn compose(self, ui: &mut Ui<'_>) -> ComposedLayout {
        let mut responses = vec![None; self.widgets.len()];
        for (index, widget) in self.widgets.into_iter().enumerate() {
            if let Some(widget) = widget {
                let rect = self.solution.rect(self.slot_nodes[index]);
                responses[index] = widget.compose(ui, rect);
            }
        }
        ComposedLayout { responses }
    }
}

/// Responses produced by [`UiLayout::compose`], indexed by slot.
pub struct ComposedLayout {
    responses: Vec<Option<Response>>,
}

impl ComposedLayout {
    /// Returns the response for a slot, if the slot held an interactive widget.
    #[must_use]
    pub fn response(&self, slot: LayoutSlot) -> Option<&Response> {
        self.responses.get(slot.0).and_then(Option::as_ref)
    }
}

impl<'a> Ui<'a> {
    /// Builds and solves a layout tree against `bounds` (RFC 0001 L1 seam).
    ///
    /// The closure declares content through a [`LayoutScope`]; widgets measure
    /// eagerly during declaration. The returned [`UiLayout`] carries final
    /// screen-space geometry, so pointer targets can be declared from
    /// [`UiLayout::rect`] before [`UiLayout::compose`] runs any behavior.
    ///
    /// The closure's content becomes a single implicit root: one child is the
    /// root itself; multiple children wrap in a zero-spacing column.
    ///
    /// # Panics
    ///
    /// Panics only if the internal scope stack invariant is violated, which
    /// would be a bug in this module rather than in caller code.
    pub fn layout<'f>(
        &mut self,
        bounds: Rect,
        build: impl FnOnce(&mut LayoutScope<'_, 'a, 'f>),
    ) -> UiLayout<'f> {
        let mut scope = LayoutScope::new(self);
        build(&mut scope);
        let LayoutScope {
            mut tree,
            widgets,
            slot_nodes,
            mut children_stack,
            ..
        } = scope;
        let roots = children_stack
            .pop()
            .expect("layout scope stack is never empty");
        let root = single_child(&mut tree, roots);
        tree.set_root(root);
        let solution = tree.solve(bounds);
        UiLayout {
            solution,
            slot_nodes,
            widgets,
        }
    }
}
