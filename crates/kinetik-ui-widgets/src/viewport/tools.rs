#[allow(clippy::wildcard_imports)]
use super::*;

/// Stable identity for a viewport overlay target declared by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportOverlayId(u64);

impl ViewportOverlayId {
    /// Creates a viewport overlay ID from raw bits.
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

/// Stable identity for a viewport guide supplied by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportGuideId(u64);

impl ViewportGuideId {
    /// Creates a viewport guide ID from raw bits.
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

/// Stable identity for a viewport safe area supplied by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportSafeAreaId(u64);

impl ViewportSafeAreaId {
    /// Creates a viewport safe-area ID from raw bits.
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

/// Stable identity for a viewport ruler overlay supplied by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportRulerId(u64);

impl ViewportRulerId {
    /// Creates a viewport ruler ID from raw bits.
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

/// Backend-neutral cursor shape requested by viewport tools or overlay targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ViewportCursorShape {
    /// Default platform cursor.
    Default,
    /// Pointer/action cursor.
    Pointer,
    /// Crosshair cursor.
    Crosshair,
    /// Open-hand grab cursor.
    Grab,
    /// Closed-hand grabbing cursor.
    Grabbing,
    /// Text edit cursor.
    Text,
    /// Move cursor.
    Move,
    /// Horizontal resize cursor.
    ResizeHorizontal,
    /// Vertical resize cursor.
    ResizeVertical,
    /// Top-left to bottom-right diagonal resize cursor.
    ResizeTopLeftBottomRight,
    /// Top-right to bottom-left diagonal resize cursor.
    ResizeTopRightBottomLeft,
    /// Rotate cursor.
    Rotate,
    /// Application-defined cursor token interpreted outside the toolkit.
    Custom(String),
}

/// Data-only cursor request metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewportCursorMetadata {
    /// Requested cursor shape.
    pub shape: ViewportCursorShape,
    /// Optional accessible/debug label for the cursor request.
    pub label: Option<String>,
}

impl ViewportCursorMetadata {
    /// Creates cursor metadata with no label.
    #[must_use]
    pub fn new(shape: ViewportCursorShape) -> Self {
        Self { shape, label: None }
    }

    /// Adds a cursor request label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Stable identity for a viewport selection target supplied by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportSelectionTargetId(u64);

impl ViewportSelectionTargetId {
    /// Creates a viewport selection target ID from raw bits.
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

/// Generic 2D transform handle kind for viewport selection targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ViewportTransformHandleKind {
    /// Move the selected target.
    Move,
    /// Resize from the top-left corner.
    ResizeTopLeft,
    /// Resize from the top edge.
    ResizeTop,
    /// Resize from the top-right corner.
    ResizeTopRight,
    /// Resize from the right edge.
    ResizeRight,
    /// Resize from the bottom-right corner.
    ResizeBottomRight,
    /// Resize from the bottom edge.
    ResizeBottom,
    /// Resize from the bottom-left corner.
    ResizeBottomLeft,
    /// Resize from the left edge.
    ResizeLeft,
    /// Rotate around the target's pivot.
    Rotate,
    /// Move the target pivot or anchor.
    Pivot,
}

impl ViewportTransformHandleKind {
    /// Returns the backend-neutral cursor shape normally associated with this handle.
    #[must_use]
    pub fn cursor_shape(self) -> ViewportCursorShape {
        match self {
            Self::Move => ViewportCursorShape::Move,
            Self::ResizeTopLeft | Self::ResizeBottomRight => {
                ViewportCursorShape::ResizeTopLeftBottomRight
            }
            Self::ResizeTop | Self::ResizeBottom => ViewportCursorShape::ResizeVertical,
            Self::ResizeTopRight | Self::ResizeBottomLeft => {
                ViewportCursorShape::ResizeTopRightBottomLeft
            }
            Self::ResizeRight | Self::ResizeLeft => ViewportCursorShape::ResizeHorizontal,
            Self::Rotate => ViewportCursorShape::Rotate,
            Self::Pivot => ViewportCursorShape::Crosshair,
        }
    }

    pub(crate) const fn hit_priority(self) -> i32 {
        match self {
            Self::ResizeTopLeft
            | Self::ResizeTopRight
            | Self::ResizeBottomRight
            | Self::ResizeBottomLeft => 90,
            Self::Rotate => 80,
            Self::Pivot => 70,
            Self::ResizeTop | Self::ResizeRight | Self::ResizeBottom | Self::ResizeLeft => 60,
            Self::Move => 10,
        }
    }
}

/// Stable identity for a transform handle on an application-supplied selection target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportTransformHandleId {
    /// Stable selection target identity.
    pub target: ViewportSelectionTargetId,
    /// Stable handle kind on the target.
    pub kind: ViewportTransformHandleKind,
}

impl ViewportTransformHandleId {
    /// Creates a stable transform handle identity.
    #[must_use]
    pub const fn new(target: ViewportSelectionTargetId, kind: ViewportTransformHandleKind) -> Self {
        Self { target, kind }
    }
}

/// Set of generic 2D transform handles exposed for a selection target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportTransformHandleSet {
    bits: u16,
}

impl ViewportTransformHandleSet {
    const MOVE: u16 = 1 << 0;
    const RESIZE_EDGES: u16 = 1 << 1;
    const RESIZE_CORNERS: u16 = 1 << 2;
    const ROTATE: u16 = 1 << 3;
    const PIVOT: u16 = 1 << 4;

    /// Creates a handle set containing every generic 2D handle.
    #[must_use]
    pub const fn all_2d() -> Self {
        Self {
            bits: Self::MOVE
                | Self::RESIZE_EDGES
                | Self::RESIZE_CORNERS
                | Self::ROTATE
                | Self::PIVOT,
        }
    }

    /// Creates a handle set containing only the move handle.
    #[must_use]
    pub const fn move_only() -> Self {
        Self { bits: Self::MOVE }
    }

    /// Returns true when this set contains the requested handle kind.
    #[must_use]
    pub const fn contains(self, kind: ViewportTransformHandleKind) -> bool {
        let bit = match kind {
            ViewportTransformHandleKind::Move => Self::MOVE,
            ViewportTransformHandleKind::ResizeTop
            | ViewportTransformHandleKind::ResizeRight
            | ViewportTransformHandleKind::ResizeBottom
            | ViewportTransformHandleKind::ResizeLeft => Self::RESIZE_EDGES,
            ViewportTransformHandleKind::ResizeTopLeft
            | ViewportTransformHandleKind::ResizeTopRight
            | ViewportTransformHandleKind::ResizeBottomRight
            | ViewportTransformHandleKind::ResizeBottomLeft => Self::RESIZE_CORNERS,
            ViewportTransformHandleKind::Rotate => Self::ROTATE,
            ViewportTransformHandleKind::Pivot => Self::PIVOT,
        };
        self.bits & bit != 0
    }

    pub(crate) fn kinds(self) -> impl Iterator<Item = ViewportTransformHandleKind> {
        [
            ViewportTransformHandleKind::Move,
            ViewportTransformHandleKind::ResizeTopLeft,
            ViewportTransformHandleKind::ResizeTop,
            ViewportTransformHandleKind::ResizeTopRight,
            ViewportTransformHandleKind::ResizeRight,
            ViewportTransformHandleKind::ResizeBottomRight,
            ViewportTransformHandleKind::ResizeBottom,
            ViewportTransformHandleKind::ResizeBottomLeft,
            ViewportTransformHandleKind::ResizeLeft,
            ViewportTransformHandleKind::Rotate,
            ViewportTransformHandleKind::Pivot,
        ]
        .into_iter()
        .filter(move |kind| self.contains(*kind))
    }
}

impl Default for ViewportTransformHandleSet {
    fn default() -> Self {
        Self::all_2d()
    }
}

/// Compact selection and interactivity state for a viewport selection target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportSelectionTargetState {
    bits: u8,
}

impl ViewportSelectionTargetState {
    const SELECTED: u8 = 1 << 0;
    const ENABLED: u8 = 1 << 1;
    const AVAILABLE: u8 = 1 << 2;
    const READ_ONLY: u8 = 1 << 3;

    /// Creates selected, enabled, available target state.
    #[must_use]
    pub const fn interactive_selected() -> Self {
        Self {
            bits: Self::SELECTED | Self::ENABLED | Self::AVAILABLE,
        }
    }

    /// Returns true when the target is part of the current selection.
    #[must_use]
    pub const fn selected(self) -> bool {
        self.bits & Self::SELECTED != 0
    }

    /// Returns true when the target can currently emit interaction requests.
    #[must_use]
    pub const fn enabled(self) -> bool {
        self.bits & Self::ENABLED != 0
    }

    /// Returns true when the target exists in the current application context.
    #[must_use]
    pub const fn available(self) -> bool {
        self.bits & Self::AVAILABLE != 0
    }

    /// Returns true when transform requests should be suppressed.
    #[must_use]
    pub const fn read_only(self) -> bool {
        self.bits & Self::READ_ONLY != 0
    }

    /// Returns state with the selected flag changed.
    #[must_use]
    pub const fn with_selected(mut self, selected: bool) -> Self {
        self.set_flag(Self::SELECTED, selected);
        self
    }

    /// Returns state with the enabled flag changed.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.set_flag(Self::ENABLED, enabled);
        self
    }

    /// Returns state with the available flag changed.
    #[must_use]
    pub const fn with_available(mut self, available: bool) -> Self {
        self.set_flag(Self::AVAILABLE, available);
        self
    }

    /// Returns state with the read-only flag changed.
    #[must_use]
    pub const fn with_read_only(mut self, read_only: bool) -> Self {
        self.set_flag(Self::READ_ONLY, read_only);
        self
    }

    const fn set_flag(&mut self, flag: u8, enabled: bool) {
        if enabled {
            self.bits |= flag;
        } else {
            self.bits &= !flag;
        }
    }
}

impl Default for ViewportSelectionTargetState {
    fn default() -> Self {
        Self::interactive_selected()
    }
}

/// Data-only viewport tool descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportToolDescriptor {
    /// Stable tool identity.
    pub id: ViewportToolId,
    /// Tool label.
    pub label: String,
    /// Tool cursor request metadata.
    pub cursor: Option<ViewportCursorMetadata>,
    /// Whether this tool is currently active.
    pub active: bool,
    /// Whether this tool can currently emit interaction requests.
    pub enabled: bool,
    /// Whether this tool is available in the current app context.
    pub available: bool,
}

impl ViewportToolDescriptor {
    /// Creates an available, enabled viewport tool descriptor.
    #[must_use]
    pub fn new(id: ViewportToolId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            cursor: None,
            active: false,
            enabled: true,
            available: true,
        }
    }

    /// Marks the tool as active.
    #[must_use]
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Marks the tool as enabled or disabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Marks the tool as available or unavailable in the current app context.
    #[must_use]
    pub fn available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }

    /// Adds cursor request metadata.
    #[must_use]
    pub fn with_cursor(mut self, cursor: ViewportCursorMetadata) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Returns true when the tool may participate in interaction routing.
    #[must_use]
    pub const fn can_interact(&self) -> bool {
        self.active && self.enabled && self.available
    }

    /// Returns cursor request metadata when this active tool can interact.
    #[must_use]
    pub fn cursor_request(&self) -> Option<&ViewportCursorMetadata> {
        self.can_interact().then_some(())?;
        self.cursor.as_ref()
    }
}

/// Semantic descriptor for a viewport tool surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportToolSurfaceDescriptor {
    /// Stable viewport widget identity.
    pub id: WidgetId,
    /// Accessible viewport label.
    pub label: String,
    /// Optional active tool metadata.
    pub active_tool: Option<ViewportToolDescriptor>,
}

impl ViewportToolSurfaceDescriptor {
    /// Creates a semantic descriptor for a viewport tool surface.
    #[must_use]
    pub fn new(id: WidgetId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            active_tool: None,
        }
    }

    /// Adds active tool metadata without executing tool behavior.
    #[must_use]
    pub fn with_active_tool(mut self, tool: ViewportToolDescriptor) -> Self {
        self.active_tool = Some(tool);
        self
    }

    /// Builds backend-neutral semantic metadata for the viewport surface.
    #[must_use]
    pub fn semantics(&self, surface: ViewportSurface) -> SemanticNode {
        let mut node =
            SemanticNode::new(self.id, SemanticRole::Viewport, surface.effective_bounds())
                .with_label(self.label.clone())
                .focusable(true);
        if let Some(tool) = &self.active_tool {
            node.children
                .push(viewport_tool_widget_id(self.id, tool.id));
            node.state.disabled = !tool.enabled || !tool.available;
            node.state.value = Some(SemanticValue::Text(format!(
                "Active tool {}: {}",
                tool.id.raw(),
                tool.label
            )));
        }
        node
    }

    /// Builds backend-neutral semantic metadata for the active tool, when present.
    #[must_use]
    pub fn active_tool_semantics(&self, surface: ViewportSurface) -> Option<SemanticNode> {
        let tool = self.active_tool.as_ref()?;
        Some(viewport_tool_semantics(self.id, surface, tool))
    }
}

/// Returns the stable semantic widget ID for a viewport tool.
#[must_use]
pub fn viewport_tool_widget_id(root: WidgetId, tool: ViewportToolId) -> WidgetId {
    root.child(("viewport-tool", tool.raw()))
}

/// Returns the stable semantic widget ID for a viewport overlay target.
#[must_use]
pub fn viewport_overlay_widget_id(root: WidgetId, overlay: ViewportOverlayId) -> WidgetId {
    root.child(("viewport-overlay", overlay.raw()))
}

/// Returns the stable semantic widget ID for a viewport guide.
#[must_use]
pub fn viewport_guide_widget_id(root: WidgetId, guide: ViewportGuideId) -> WidgetId {
    root.child(("viewport-guide", guide.raw()))
}

/// Returns the stable semantic widget ID for a viewport safe area.
#[must_use]
pub fn viewport_safe_area_widget_id(root: WidgetId, safe_area: ViewportSafeAreaId) -> WidgetId {
    root.child(("viewport-safe-area", safe_area.raw()))
}

/// Returns the stable semantic widget ID for a viewport ruler.
#[must_use]
pub fn viewport_ruler_widget_id(root: WidgetId, ruler: ViewportRulerId) -> WidgetId {
    root.child(("viewport-ruler", ruler.raw()))
}

/// Builds backend-neutral semantic metadata for a viewport tool.
#[must_use]
pub fn viewport_tool_semantics(
    root: WidgetId,
    surface: ViewportSurface,
    tool: &ViewportToolDescriptor,
) -> SemanticNode {
    let mut node = SemanticNode::new(
        viewport_tool_widget_id(root, tool.id),
        SemanticRole::Custom("viewport-tool".to_owned()),
        surface.effective_bounds(),
    )
    .with_label(tool.label.clone())
    .focusable(tool.enabled && tool.available);
    node.state.selected = tool.active;
    node.state.disabled = !tool.enabled || !tool.available;
    node.state.value = Some(SemanticValue::Text(format!(
        "Tool {}: {}",
        tool.id.raw(),
        tool.label
    )));
    node
}
