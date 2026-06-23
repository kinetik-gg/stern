//! UI frame runtime boundary types.

use std::hash::Hash;
use std::time::Duration;

use crate::input::{Key, KeyEvent, KeyState, UiInput};
use crate::memory::UiMemory;
use crate::render::{ClipId, LayerId, Primitive};
use crate::{
    AccessibilitySnapshot, ActionContext, ActionId, ActionInvocation, ActionQueue, ActionSource,
    FocusTraversal, PhysicalSize, Rect, ScaleFactor, SemanticNode, SemanticTree, SemanticTreeError,
    Size, WidgetId,
};
use crate::{IdStack, Transform};

/// Information about the current rendering viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportInfo {
    /// Size used by UI layout.
    pub logical_size: Size,
    /// Size of the physical render target.
    pub physical_size: PhysicalSize,
    /// Scale factor between logical and physical units.
    pub scale_factor: ScaleFactor,
}

impl ViewportInfo {
    /// Creates viewport information.
    #[must_use]
    pub const fn new(
        logical_size: Size,
        physical_size: PhysicalSize,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            logical_size,
            physical_size,
            scale_factor,
        }
    }
}

/// Time information for one UI frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TimeInfo {
    /// Monotonic timestamp relative to the application-defined start.
    pub now: Duration,
    /// Time since the previous frame.
    pub delta: Duration,
    /// Sequential frame number.
    pub frame_index: u64,
}

impl TimeInfo {
    /// Creates frame time information.
    #[must_use]
    pub const fn new(now: Duration, delta: Duration, frame_index: u64) -> Self {
        Self {
            now,
            delta,
            frame_index,
        }
    }
}

/// Context provided to the UI runtime at the beginning of a frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameContext {
    /// Viewport and DPI information.
    pub viewport: ViewportInfo,
    /// Input snapshot for this frame.
    pub input: UiInput,
    /// Time snapshot for this frame.
    pub time: TimeInfo,
}

impl FrameContext {
    /// Creates a frame context.
    #[must_use]
    pub const fn new(viewport: ViewportInfo, input: UiInput, time: TimeInfo) -> Self {
        Self {
            viewport,
            input,
            time,
        }
    }
}

/// Request for when the platform adapter should schedule another redraw.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RepaintRequest {
    /// No repaint is currently needed.
    #[default]
    None,
    /// Repaint as soon as the platform can present another frame.
    NextFrame,
    /// Repaint after the provided delay.
    After(Duration),
    /// Continue repainting while an external active condition remains true.
    Continuous,
}

impl RepaintRequest {
    /// Combines two repaint requests, preserving the more urgent request.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Continuous, _) | (_, Self::Continuous) => Self::Continuous,
            (Self::NextFrame, _) | (_, Self::NextFrame) => Self::NextFrame,
            (Self::After(a), Self::After(b)) => Self::After(a.min(b)),
            (Self::After(delay), Self::None) | (Self::None, Self::After(delay)) => {
                Self::After(delay)
            }
            (Self::None, Self::None) => Self::None,
        }
    }
}

/// Cursor shape requested by toolkit code.
///
/// Platform adapters translate these neutral shapes to the host cursor API.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CursorShape {
    /// Platform default cursor.
    #[default]
    Default,
    /// Text insertion cursor.
    Text,
    /// Clickable item cursor.
    PointingHand,
    /// Crosshair cursor.
    Crosshair,
    /// Open hand drag cursor.
    Grab,
    /// Closed hand drag cursor.
    Grabbing,
    /// Horizontal resize cursor.
    ResizeHorizontal,
    /// Vertical resize cursor.
    ResizeVertical,
    /// Diagonal resize from top-left to bottom-right.
    ResizeTopLeftBottomRight,
    /// Diagonal resize from top-right to bottom-left.
    ResizeTopRightBottomLeft,
    /// Operation is unavailable.
    NotAllowed,
}

/// Platform-neutral request emitted by toolkit code during a frame.
///
/// The core crate records intent only. Windowing, clipboard, IME, browser, and
/// shell integration stay in platform/application adapters.
#[derive(Debug, Clone, PartialEq)]
pub enum PlatformRequest {
    /// Set the pointer cursor for the current frame.
    SetCursor(CursorShape),
    /// Copy text to the platform clipboard.
    CopyToClipboard(String),
    /// Ask the platform adapter to provide clipboard text as future input.
    RequestClipboardText {
        /// Text-input widget that should receive the clipboard text.
        target: WidgetId,
    },
    /// Start platform text input or IME at an optional logical text-editing rect.
    StartTextInput {
        /// Logical rectangle for caret/composition placement.
        rect: Option<Rect>,
    },
    /// Stop platform text input or IME.
    StopTextInput,
    /// Set the host window title.
    SetWindowTitle(String),
    /// Ask the application/platform shell to open a URL.
    OpenUrl(String),
}

/// Runtime warning detected while finalizing a UI frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameWarning {
    /// The same widget ID was registered more than once in one frame.
    DuplicateWidgetId {
        /// Duplicated widget identity.
        id: WidgetId,
    },
    /// A clip end command did not match the current open clip.
    UnmatchedClipEnd {
        /// Clip ID carried by the unmatched end command.
        id: ClipId,
    },
    /// A clip begin command remained open at the end of the frame.
    UnclosedClip {
        /// Clip ID left open.
        id: ClipId,
    },
    /// A layer end command did not match the current open layer.
    UnmatchedLayerEnd {
        /// Layer ID carried by the unmatched end command.
        id: LayerId,
    },
    /// A layer begin command remained open at the end of the frame.
    UnclosedLayer {
        /// Layer ID left open.
        id: LayerId,
    },
    /// A transform end command appeared without a matching begin.
    UnmatchedTransformEnd,
    /// Transform begin commands remained open at the end of the frame.
    UnclosedTransforms {
        /// Number of unclosed transform scopes.
        count: usize,
    },
    /// Accessibility semantic tree failed structural validation.
    InvalidSemanticTree {
        /// Structural validation error.
        error: SemanticTreeError,
    },
}

/// Output produced by a UI frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameOutput {
    /// Backend-independent render primitives.
    pub primitives: Vec<Primitive>,
    /// Accessibility semantic tree for the frame.
    pub semantics: SemanticTree,
    /// Repaint scheduling request.
    pub repaint: RepaintRequest,
    /// Action invocations emitted during the frame.
    pub actions: ActionQueue,
    /// Requests for platform/application adapters.
    pub platform_requests: Vec<PlatformRequest>,
    /// Diagnostics detected while building or finalizing the frame.
    pub warnings: Vec<FrameWarning>,
}

impl FrameOutput {
    /// Creates empty frame output.
    #[must_use]
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            semantics: SemanticTree::new(),
            repaint: RepaintRequest::None,
            actions: ActionQueue::new(),
            platform_requests: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Appends one render primitive.
    pub fn push_primitive(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    /// Appends render primitives in order.
    pub fn extend_primitives(&mut self, primitives: impl IntoIterator<Item = Primitive>) {
        self.primitives.extend(primitives);
    }

    /// Sets the semantic root node.
    pub fn set_semantic_root(&mut self, root: WidgetId) {
        self.semantics.set_root(root);
    }

    /// Appends one semantic node in traversal order.
    pub fn push_semantic_node(&mut self, node: SemanticNode) {
        self.semantics.push(node);
    }

    /// Requests repaint scheduling.
    pub fn request_repaint(&mut self, request: RepaintRequest) {
        self.repaint = self.repaint.merge(request);
    }

    /// Adds an action invocation to the frame output.
    pub fn push_action(&mut self, invocation: ActionInvocation) {
        self.actions.push(invocation);
        self.request_repaint(RepaintRequest::NextFrame);
    }

    /// Adds an action invocation from simple parts.
    pub fn invoke_action(
        &mut self,
        action_id: ActionId,
        source: ActionSource,
        context: ActionContext,
    ) {
        self.actions.invoke(action_id, source, context);
        self.request_repaint(RepaintRequest::NextFrame);
    }

    /// Appends one platform request.
    pub fn push_platform_request(&mut self, request: PlatformRequest) {
        self.platform_requests.push(request);
    }

    /// Appends one runtime warning.
    pub fn push_warning(&mut self, warning: FrameWarning) {
        self.warnings.push(warning);
    }

    /// Exports a validated accessibility snapshot for platform adapters.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticTreeError`] when the frame's semantic tree is
    /// structurally invalid.
    pub fn accessibility_snapshot(
        &self,
        focused: Option<WidgetId>,
    ) -> Result<AccessibilitySnapshot, SemanticTreeError> {
        self.semantics.accessibility_snapshot(focused)
    }
}

/// Frame-local UI runtime builder.
///
/// This type owns stable ID derivation and the frame output accumulator. Widget
/// crates can layer ergonomic component APIs on top without becoming the
/// lowest-level runtime abstraction.
pub struct Ui<'a> {
    context: FrameContext,
    memory: &'a mut UiMemory,
    ids: IdStack,
    output: FrameOutput,
}

impl<'a> Ui<'a> {
    /// Starts a UI frame and clears transient retained memory.
    #[must_use]
    pub fn begin_frame(context: FrameContext, memory: &'a mut UiMemory) -> Self {
        memory.begin_frame();
        if !context.input.window_focused || pointer_release_all_cancelled(&context.input) {
            memory.cancel_pointer_interaction();
        }
        Self {
            context,
            memory,
            ids: IdStack::new(),
            output: FrameOutput::new(),
        }
    }

    /// Returns the frame context.
    #[must_use]
    pub const fn context(&self) -> &FrameContext {
        &self.context
    }

    /// Returns the input snapshot.
    #[must_use]
    pub const fn input(&self) -> &UiInput {
        &self.context.input
    }

    /// Returns retained UI memory.
    #[must_use]
    pub fn memory(&self) -> &UiMemory {
        self.memory
    }

    /// Returns mutable retained UI memory.
    pub fn memory_mut(&mut self) -> &mut UiMemory {
        self.memory
    }

    /// Returns input and mutable memory as separate borrows for widget composition.
    pub fn input_and_memory_mut(&mut self) -> (&UiInput, &mut UiMemory) {
        (&self.context.input, self.memory)
    }

    /// Derives and registers a widget ID in the current scope.
    pub fn id(&mut self, key: impl Hash) -> WidgetId {
        self.ids.register_key(key)
    }

    /// Registers an externally derived widget ID for duplicate detection.
    pub fn register_id(&mut self, id: WidgetId) -> WidgetId {
        self.ids.register(id);
        id
    }

    /// Pushes a stable ID scope and returns the scope ID.
    pub fn push_id_scope(&mut self, key: impl Hash) -> WidgetId {
        self.ids.push(key)
    }

    /// Pops the current stable ID scope.
    pub fn pop_id_scope(&mut self) -> Option<WidgetId> {
        self.ids.pop()
    }

    /// Runs a closure inside a stable ID scope.
    pub fn scope<T>(&mut self, key: impl Hash, f: impl FnOnce(&mut Self) -> T) -> T {
        self.ids.push(key);
        let result = f(self);
        self.ids.pop();
        result
    }

    /// Returns the accumulated output so far.
    #[must_use]
    pub const fn output(&self) -> &FrameOutput {
        &self.output
    }

    /// Appends one render primitive.
    pub fn push_primitive(&mut self, primitive: Primitive) {
        self.output.push_primitive(primitive);
    }

    /// Appends render primitives in order.
    pub fn extend_primitives(&mut self, primitives: impl IntoIterator<Item = Primitive>) {
        self.output.extend_primitives(primitives);
    }

    /// Sets the semantic root node.
    pub fn set_semantic_root(&mut self, root: WidgetId) {
        self.output.set_semantic_root(root);
    }

    /// Appends one semantic node in traversal order.
    pub fn push_semantic_node(&mut self, node: SemanticNode) {
        self.output.push_semantic_node(node);
    }

    /// Requests repaint scheduling.
    pub fn request_repaint(&mut self, request: RepaintRequest) {
        self.output.request_repaint(request);
    }

    /// Adds an action invocation to the frame output.
    pub fn push_action(&mut self, invocation: ActionInvocation) {
        self.output.push_action(invocation);
    }

    /// Adds an action invocation from simple parts.
    pub fn invoke_action(
        &mut self,
        action_id: ActionId,
        source: ActionSource,
        context: ActionContext,
    ) {
        self.output.invoke_action(action_id, source, context);
    }

    /// Appends one platform request.
    pub fn push_platform_request(&mut self, request: PlatformRequest) {
        self.output.push_platform_request(request);
    }

    /// Appends one runtime warning.
    pub fn push_warning(&mut self, warning: FrameWarning) {
        self.output.push_warning(warning);
    }

    /// Finishes the frame and returns deterministic output.
    #[must_use]
    pub fn end_frame(mut self) -> FrameOutput {
        for duplicate in self.ids.duplicates() {
            self.output
                .push_warning(FrameWarning::DuplicateWidgetId { id: duplicate.id });
        }

        let semantic_tree_valid = match self.output.semantics.validate() {
            Ok(()) => true,
            Err(error) => {
                self.output
                    .push_warning(FrameWarning::InvalidSemanticTree { error });
                false
            }
        };
        if semantic_tree_valid
            && apply_keyboard_focus_traversal(
                &self.context.input,
                self.memory,
                &self.output.semantics,
            )
        {
            self.output.request_repaint(RepaintRequest::NextFrame);
        }
        if self.memory.take_pending_text_input_stop().is_some() {
            self.output
                .push_platform_request(PlatformRequest::StopTextInput);
        }

        let warnings = validate_primitive_stack(&self.output.primitives);
        self.output.warnings.extend(warnings);
        self.output
    }
}

fn pointer_release_all_cancelled(input: &UiInput) -> bool {
    input.pointer.release_all_cancelled()
}

fn apply_keyboard_focus_traversal(
    input: &UiInput,
    memory: &mut UiMemory,
    semantics: &SemanticTree,
) -> bool {
    if memory.text_input_owner().is_some() {
        return false;
    }

    let directions: Vec<_> = input
        .keyboard
        .events
        .iter()
        .filter_map(tab_focus_direction)
        .collect();
    if directions.is_empty() {
        return false;
    }

    let order = semantics.focus_order();
    if order.is_empty() {
        return false;
    }

    let mut focused = memory.focused().filter(|id| order.contains(id));
    let initial = focused;
    for direction in directions {
        let traversal = FocusTraversal {
            order: order.clone(),
            focused,
        };
        focused = match direction {
            TabFocusDirection::Forward => traversal.next(),
            TabFocusDirection::Backward => traversal.previous(),
        };
    }

    if focused == initial {
        return false;
    }

    memory.set_focused(focused);
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TabFocusDirection {
    Forward,
    Backward,
}

fn tab_focus_direction(event: &KeyEvent) -> Option<TabFocusDirection> {
    if event.state != KeyState::Pressed || event.repeat || !matches!(event.key, Key::Tab) {
        return None;
    }

    if event.modifiers.is_empty() {
        Some(TabFocusDirection::Forward)
    } else if event.modifiers.shift
        && !event.modifiers.ctrl
        && !event.modifiers.alt
        && !event.modifiers.super_key
    {
        Some(TabFocusDirection::Backward)
    } else {
        None
    }
}

fn validate_primitive_stack(primitives: &[Primitive]) -> Vec<FrameWarning> {
    let mut warnings = Vec::new();
    let mut scopes = Vec::new();

    for primitive in primitives {
        match primitive {
            Primitive::ClipBegin { id, .. } => scopes.push(PrimitiveScope::Clip(*id)),
            Primitive::ClipEnd { id } => match scopes.last().copied() {
                Some(PrimitiveScope::Clip(open_id)) if open_id == *id => {
                    scopes.pop();
                }
                _ => warnings.push(FrameWarning::UnmatchedClipEnd { id: *id }),
            },
            Primitive::LayerBegin { id } => scopes.push(PrimitiveScope::Layer(*id)),
            Primitive::LayerEnd { id } => match scopes.last().copied() {
                Some(PrimitiveScope::Layer(open_id)) if open_id == *id => {
                    scopes.pop();
                }
                _ => warnings.push(FrameWarning::UnmatchedLayerEnd { id: *id }),
            },
            Primitive::TransformBegin(Transform { .. }) => {
                scopes.push(PrimitiveScope::Transform);
            }
            Primitive::TransformEnd => match scopes.last().copied() {
                Some(PrimitiveScope::Transform) => {
                    scopes.pop();
                }
                _ => warnings.push(FrameWarning::UnmatchedTransformEnd),
            },
            Primitive::Rect(_)
            | Primitive::Line(_)
            | Primitive::Shadow(_)
            | Primitive::Path(_)
            | Primitive::Text(_)
            | Primitive::Image(_)
            | Primitive::Texture(_) => {}
        }
    }

    let mut clips = Vec::new();
    let mut layers = Vec::new();
    let mut transform_depth = 0;
    for scope in scopes {
        match scope {
            PrimitiveScope::Clip(id) => clips.push(id),
            PrimitiveScope::Layer(id) => layers.push(id),
            PrimitiveScope::Transform => transform_depth += 1,
        }
    }

    warnings.extend(
        clips
            .into_iter()
            .rev()
            .map(|id| FrameWarning::UnclosedClip { id }),
    );
    warnings.extend(
        layers
            .into_iter()
            .rev()
            .map(|id| FrameWarning::UnclosedLayer { id }),
    );
    if transform_depth > 0 {
        warnings.push(FrameWarning::UnclosedTransforms {
            count: transform_depth,
        });
    }

    warnings
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrimitiveScope {
    Clip(ClipId),
    Layer(LayerId),
    Transform,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::time::Duration;

    use super::{
        CursorShape, FrameContext, FrameOutput, FrameWarning, PlatformRequest, RepaintRequest,
        TimeInfo, Ui, ViewportInfo,
    };
    use crate::input::UiInput;
    use crate::{
        ActionContext, ActionId, ActionSource, Brush, ClipId, Color, CornerRadius, LayerId,
        PhysicalSize, Primitive, Rect, RectPrimitive, ScaleFactor, SemanticNode, SemanticRole,
        SemanticTreeError, Size, Transform, UiMemory, WidgetId,
    };

    #[test]
    fn creates_viewport_info() {
        let viewport = ViewportInfo::new(
            Size::new(800.0, 600.0),
            PhysicalSize::new(1600, 1200),
            ScaleFactor::new(2.0),
        );

        assert_eq!(viewport.logical_size, Size::new(800.0, 600.0));
        assert_eq!(viewport.physical_size, PhysicalSize::new(1600, 1200));
        assert_eq!(viewport.scale_factor.value(), 2.0);
    }

    #[test]
    fn creates_frame_context() {
        let viewport = ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        );
        let time = TimeInfo::new(Duration::from_millis(16), Duration::from_millis(16), 1);
        let context = FrameContext::new(viewport, UiInput::default(), time);

        assert_eq!(context.viewport, viewport);
        assert_eq!(context.time.frame_index, 1);
    }

    #[test]
    fn frame_output_defaults_to_no_repaint() {
        let output = FrameOutput::new();

        assert!(output.primitives.is_empty());
        assert!(output.semantics.nodes().is_empty());
        assert_eq!(output.repaint, RepaintRequest::None);
        assert!(output.actions.is_empty());
        assert!(output.platform_requests.is_empty());
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn repaint_request_merge_keeps_most_urgent_request() {
        assert_eq!(
            RepaintRequest::After(Duration::from_secs(2))
                .merge(RepaintRequest::After(Duration::from_secs(1))),
            RepaintRequest::After(Duration::from_secs(1))
        );
        assert_eq!(
            RepaintRequest::After(Duration::from_secs(1)).merge(RepaintRequest::NextFrame),
            RepaintRequest::NextFrame
        );
        assert_eq!(
            RepaintRequest::NextFrame.merge(RepaintRequest::Continuous),
            RepaintRequest::Continuous
        );
    }

    #[test]
    fn frame_output_accumulates_repaint_requests() {
        let mut output = FrameOutput::new();

        output.request_repaint(RepaintRequest::After(Duration::from_secs(5)));
        output.request_repaint(RepaintRequest::After(Duration::from_secs(1)));

        assert_eq!(
            output.repaint,
            RepaintRequest::After(Duration::from_secs(1))
        );
    }

    #[test]
    fn frame_output_accumulates_actions() {
        let mut output = FrameOutput::new();

        output.invoke_action(
            ActionId::new("file.save"),
            ActionSource::Shortcut,
            ActionContext::Global,
        );

        assert_eq!(output.actions.len(), 1);
        assert_eq!(output.repaint, RepaintRequest::NextFrame);
        assert_eq!(
            output.actions.pop_front().expect("action").action_id,
            ActionId::new("file.save")
        );
    }

    #[test]
    fn frame_output_accumulates_render_semantics_and_platform_requests() {
        let mut output = FrameOutput::new();
        let id = WidgetId::from_key("button");
        let rect = Rect::new(1.0, 2.0, 30.0, 20.0);

        output.push_primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: None,
            radius: CornerRadius::all(2.0),
        }));
        output.push_semantic_node(
            SemanticNode::new(id, SemanticRole::Button, rect)
                .focusable(true)
                .with_label("Run"),
        );
        output.push_platform_request(PlatformRequest::SetCursor(CursorShape::PointingHand));

        assert_eq!(output.primitives.len(), 1);
        assert_eq!(output.semantics.root(), Some(id));
        assert_eq!(output.semantics.focus_order(), vec![id]);
        assert_eq!(
            output.platform_requests,
            vec![PlatformRequest::SetCursor(CursorShape::PointingHand)]
        );
    }

    #[test]
    fn frame_output_exports_accessibility_snapshot_independent_from_painting() {
        let mut output = FrameOutput::new();
        let root = WidgetId::from_key("root");
        let button = WidgetId::from_key("button");
        let rect = Rect::new(1.0, 2.0, 30.0, 20.0);

        output.push_primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: None,
            radius: CornerRadius::all(2.0),
        }));
        output.push_semantic_node(
            SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([button]),
        );
        output.push_semantic_node(
            SemanticNode::new(button, SemanticRole::Button, rect)
                .focusable(true)
                .with_label("Run"),
        );

        let snapshot = output
            .accessibility_snapshot(Some(button))
            .expect("snapshot");

        assert_eq!(output.primitives.len(), 1);
        assert_eq!(
            snapshot
                .nodes
                .iter()
                .map(|node| node.id)
                .collect::<Vec<_>>(),
            vec![root, button]
        );
        assert_eq!(snapshot.focus_order, vec![button]);
        assert_eq!(snapshot.focused, Some(button));
        assert_eq!(
            snapshot.node(button).expect("button").label.as_deref(),
            Some("Run")
        );
    }

    #[test]
    fn ui_builder_registers_ids_and_finalizes_output() {
        let viewport = ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        );
        let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
        let mut memory = UiMemory::new();
        memory.focus(WidgetId::from_key("stale"));

        let mut ui = Ui::begin_frame(context, &mut memory);
        let id = ui.id("save");
        let duplicate = ui.register_id(id);
        ui.push_semantic_node(SemanticNode::new(id, SemanticRole::Button, Rect::ZERO));
        ui.invoke_action(
            ActionId::new("file.save"),
            ActionSource::Button,
            ActionContext::Global,
        );
        ui.push_platform_request(PlatformRequest::SetCursor(CursorShape::PointingHand));
        let output = ui.end_frame();

        assert_eq!(id, duplicate);
        assert_eq!(output.semantics.root(), Some(id));
        assert_eq!(output.actions.len(), 1);
        assert_eq!(output.platform_requests.len(), 1);
        assert_eq!(
            output.warnings,
            vec![FrameWarning::DuplicateWidgetId { id }]
        );
    }

    #[test]
    fn ui_builder_registers_scopes_for_duplicate_detection() {
        let viewport = ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        );
        let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
        let mut memory = UiMemory::new();
        let mut ui = Ui::begin_frame(context, &mut memory);

        let id = ui.push_id_scope("panel");
        ui.pop_id_scope();
        ui.push_id_scope("panel");
        let output = ui.end_frame();

        assert_eq!(
            output.warnings,
            vec![FrameWarning::DuplicateWidgetId { id }]
        );
    }

    #[test]
    fn ui_builder_clears_transient_memory_at_frame_start() {
        let viewport = ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        );
        let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
        let mut memory = UiMemory::new();
        let hovered = WidgetId::from_key("hovered");
        let focused = WidgetId::from_key("focused");
        memory.set_hovered(hovered);
        memory.focus(focused);

        let ui = Ui::begin_frame(context, &mut memory);

        assert_eq!(ui.memory().hovered(), None);
        assert_eq!(ui.memory().focused(), Some(focused));
    }

    #[test]
    fn ui_builder_cancels_pointer_interaction_on_focus_loss_at_frame_start() {
        let viewport = ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        );
        let context = FrameContext::new(
            viewport,
            UiInput {
                window_focused: false,
                ..UiInput::default()
            },
            TimeInfo::default(),
        );
        let mut memory = UiMemory::new();
        let focused = WidgetId::from_key("focused");
        let owner = WidgetId::from_key("owner");
        memory.focus(focused);
        memory.set_text_input_owner(focused);
        memory.activate(owner);
        memory.press(owner);
        memory.capture_pointer(owner);
        memory.start_drag(owner);

        let ui = Ui::begin_frame(context, &mut memory);

        assert_eq!(ui.memory().active(), None);
        assert_eq!(ui.memory().pressed(), None);
        assert_eq!(ui.memory().pointer_capture(), None);
        assert_eq!(ui.memory().drag_source(), None);
        assert!(ui.memory().pointer_interaction_cancelled());
        assert_eq!(ui.memory().focused(), Some(focused));
        assert_eq!(ui.memory().text_input_owner(), Some(focused));
    }

    #[test]
    fn ui_builder_cancels_pointer_interaction_on_release_all_at_frame_start() {
        let viewport = ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        );
        let mut input = UiInput {
            window_focused: true,
            ..UiInput::default()
        };
        input.release_pointer_buttons();
        let context = FrameContext::new(viewport, input, TimeInfo::default());
        let mut memory = UiMemory::new();
        let focused = WidgetId::from_key("focused");
        let owner = WidgetId::from_key("owner");
        memory.focus(focused);
        memory.set_text_input_owner(focused);
        memory.activate(owner);
        memory.press(owner);
        memory.capture_pointer(owner);
        memory.start_drag(owner);

        let ui = Ui::begin_frame(context, &mut memory);

        assert_eq!(ui.memory().active(), None);
        assert_eq!(ui.memory().pressed(), None);
        assert_eq!(ui.memory().pointer_capture(), None);
        assert_eq!(ui.memory().drag_source(), None);
        assert!(ui.memory().pointer_interaction_cancelled());
        assert_eq!(ui.memory().focused(), Some(focused));
        assert_eq!(ui.memory().text_input_owner(), Some(focused));
    }

    #[test]
    fn end_frame_warns_about_unbalanced_primitive_stacks() {
        let viewport = ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        );
        let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
        let mut memory = UiMemory::new();
        let mut ui = Ui::begin_frame(context, &mut memory);
        let open_clip = ClipId::from_raw(1);
        let wrong_clip = ClipId::from_raw(2);
        let open_layer = LayerId::from_raw(3);

        ui.extend_primitives([
            Primitive::ClipBegin {
                id: open_clip,
                rect: Rect::ZERO,
            },
            Primitive::ClipEnd { id: wrong_clip },
            Primitive::LayerBegin { id: open_layer },
            Primitive::TransformEnd,
            Primitive::TransformBegin(Transform::IDENTITY),
        ]);
        let output = ui.end_frame();

        assert_eq!(
            output.warnings,
            vec![
                FrameWarning::UnmatchedClipEnd { id: wrong_clip },
                FrameWarning::UnmatchedTransformEnd,
                FrameWarning::UnclosedClip { id: open_clip },
                FrameWarning::UnclosedLayer { id: open_layer },
                FrameWarning::UnclosedTransforms { count: 1 },
            ]
        );
    }

    #[test]
    fn end_frame_warns_about_crossed_primitive_scopes() {
        let viewport = ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        );
        let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
        let mut memory = UiMemory::new();
        let mut ui = Ui::begin_frame(context, &mut memory);
        let layer = LayerId::from_raw(1);
        let clip = ClipId::from_raw(2);

        ui.extend_primitives([
            Primitive::LayerBegin { id: layer },
            Primitive::ClipBegin {
                id: clip,
                rect: Rect::ZERO,
            },
            Primitive::LayerEnd { id: layer },
            Primitive::ClipEnd { id: clip },
        ]);
        let output = ui.end_frame();

        assert_eq!(
            output.warnings,
            vec![
                FrameWarning::UnmatchedLayerEnd { id: layer },
                FrameWarning::UnclosedLayer { id: layer },
            ]
        );
    }

    #[test]
    fn end_frame_warns_about_invalid_semantic_tree() {
        let viewport = ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        );
        let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
        let mut memory = UiMemory::new();
        let mut ui = Ui::begin_frame(context, &mut memory);
        let root = WidgetId::from_key("root");
        let missing = WidgetId::from_key("missing");

        ui.push_semantic_node(
            SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([missing]),
        );
        let output = ui.end_frame();

        assert_eq!(
            output.warnings,
            vec![FrameWarning::InvalidSemanticTree {
                error: SemanticTreeError::UnknownChild {
                    parent: root,
                    child: missing,
                }
            }]
        );
    }
}
