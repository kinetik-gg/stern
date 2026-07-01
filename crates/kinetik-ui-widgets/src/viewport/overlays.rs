#[allow(clippy::wildcard_imports)]
use super::*;

/// Application-supplied data-only selection target descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportSelectionTargetDescriptor {
    /// Stable target identity supplied by the application.
    pub id: ViewportSelectionTargetId,
    /// Target bounds in viewport content coordinates.
    pub content_rect: Rect,
    /// Selection and interactivity state.
    pub state: ViewportSelectionTargetState,
    /// Explicit target priority. Higher priority is treated as visually topmost.
    pub priority: i32,
    /// Generic 2D transform handles exposed for this target.
    pub handles: ViewportTransformHandleSet,
    /// Logical handle size in screen-space units.
    pub handle_size: f32,
    /// Logical distance from the top edge to the rotate handle center.
    pub rotate_offset: f32,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportSelectionTargetDescriptor {
    /// Creates a selected, enabled, available target descriptor with all generic 2D handles.
    #[must_use]
    pub fn new(id: ViewportSelectionTargetId, content_rect: Rect) -> Self {
        Self {
            id,
            content_rect,
            state: ViewportSelectionTargetState::interactive_selected(),
            priority: 0,
            handles: ViewportTransformHandleSet::all_2d(),
            handle_size: 9.0,
            rotate_offset: 20.0,
            label: None,
        }
    }

    /// Marks the target as selected or unselected.
    #[must_use]
    pub const fn selected(mut self, selected: bool) -> Self {
        self.state = self.state.with_selected(selected);
        self
    }

    /// Marks the target as enabled or disabled.
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.state = self.state.with_enabled(enabled);
        self
    }

    /// Marks the target as available or unavailable in the current app context.
    #[must_use]
    pub const fn available(mut self, available: bool) -> Self {
        self.state = self.state.with_available(available);
        self
    }

    /// Marks the target as read-only.
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.state = self.state.with_read_only(read_only);
        self
    }

    /// Sets explicit hit-test priority. Higher priority is topmost.
    #[must_use]
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Sets the generic transform handles exposed for this target.
    #[must_use]
    pub const fn with_handles(mut self, handles: ViewportTransformHandleSet) -> Self {
        self.handles = handles;
        self
    }

    /// Sets logical transform handle size in screen-space units.
    #[must_use]
    pub const fn with_handle_size(mut self, handle_size: f32) -> Self {
        self.handle_size = handle_size;
        self
    }

    /// Sets logical rotate handle offset from the target top edge.
    #[must_use]
    pub const fn with_rotate_offset(mut self, rotate_offset: f32) -> Self {
        self.rotate_offset = rotate_offset;
        self
    }

    /// Adds an accessible/debug label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Returns true when the target can expose selection outline metadata.
    #[must_use]
    pub const fn can_show_selection(&self) -> bool {
        self.state.selected() && self.state.available()
    }

    /// Returns true when this target can emit transform handle requests.
    #[must_use]
    pub const fn can_request_transform(&self) -> bool {
        self.state.selected()
            && self.state.enabled()
            && self.state.available()
            && !self.state.read_only()
    }

    /// Returns the target bounds transformed into UI logical screen space.
    #[must_use]
    pub fn screen_rect(&self, surface: ViewportSurface, scale_factor: ScaleFactor) -> Option<Rect> {
        finite_positive_rect(self.content_rect)
            .and_then(|rect| surface.content_rect_to_screen_at(rect, scale_factor))
            .and_then(finite_positive_rect)
    }

    pub(crate) fn effective_handle_size(&self) -> f32 {
        finite_positive(self.handle_size).unwrap_or(9.0)
    }

    pub(crate) fn effective_rotate_offset(&self) -> f32 {
        finite_positive(self.rotate_offset).unwrap_or(20.0)
    }
}

/// Data-only selection outline descriptor resolved into screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportSelectionOutlineDescriptor {
    /// Stable target identity.
    pub target: ViewportSelectionTargetId,
    /// Source target bounds in content coordinates.
    pub content_rect: Rect,
    /// Selection outline rectangle in UI logical screen space.
    pub screen_rect: Rect,
    /// Whether the target can currently emit interaction requests.
    pub enabled: bool,
    /// Whether the target exists in the current application context.
    pub available: bool,
    /// Whether transform requests are suppressed for this target.
    pub read_only: bool,
    /// Target priority used for deterministic ordering.
    pub priority: i32,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportSelectionOutlineDescriptor {
    pub(crate) fn from_target(
        target: &ViewportSelectionTargetDescriptor,
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
    ) -> Option<Self> {
        target.can_show_selection().then_some(())?;
        Some(Self {
            target: target.id,
            content_rect: finite_positive_rect(target.content_rect)?,
            screen_rect: target.screen_rect(surface, scale_factor)?,
            enabled: target.state.enabled(),
            available: target.state.available(),
            read_only: target.state.read_only(),
            priority: target.priority,
            label: target.label.clone(),
        })
    }
}

/// Data-only transform handle descriptor resolved into screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportTransformHandleDescriptor {
    /// Stable handle identity.
    pub id: ViewportTransformHandleId,
    /// Stable selection target identity.
    pub target: ViewportSelectionTargetId,
    /// Generic handle kind.
    pub kind: ViewportTransformHandleKind,
    /// Source target bounds in content coordinates.
    pub source_content_rect: Rect,
    /// Source target bounds in UI logical screen space.
    pub target_screen_rect: Rect,
    /// Handle hit rectangle in UI logical screen space.
    pub handle_screen_rect: Rect,
    /// Target priority used for deterministic topmost resolution.
    pub target_priority: i32,
    /// Handle-specific hit priority.
    pub handle_priority: i32,
    /// Cursor request metadata associated with this handle.
    pub cursor: ViewportCursorMetadata,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

/// Data-only result of viewport transform handle hit testing.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportTransformHandleHit {
    /// Stable handle identity.
    pub handle: ViewportTransformHandleId,
    /// Stable selection target identity.
    pub target: ViewportSelectionTargetId,
    /// Generic handle kind.
    pub kind: ViewportTransformHandleKind,
    /// Source target bounds in content coordinates.
    pub source_content_rect: Rect,
    /// Source target bounds in UI logical screen space.
    pub target_screen_rect: Rect,
    /// Handle hit rectangle in UI logical screen space.
    pub handle_screen_rect: Rect,
    /// Hit point in UI logical screen space.
    pub point: Point,
    /// Hit point transformed into content coordinates when possible.
    pub content_point: Option<Point>,
    /// Target priority used for deterministic topmost resolution.
    pub target_priority: i32,
    /// Handle-specific hit priority.
    pub handle_priority: i32,
    /// Cursor request metadata associated with the hit handle.
    pub cursor: ViewportCursorMetadata,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportTransformHandleHit {
    pub(crate) fn from_descriptor(
        descriptor: &ViewportTransformHandleDescriptor,
        point: Point,
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            handle: descriptor.id,
            target: descriptor.target,
            kind: descriptor.kind,
            source_content_rect: descriptor.source_content_rect,
            target_screen_rect: descriptor.target_screen_rect,
            handle_screen_rect: descriptor.handle_screen_rect,
            point,
            content_point: surface.screen_to_content_at(point, scale_factor),
            target_priority: descriptor.target_priority,
            handle_priority: descriptor.handle_priority,
            cursor: descriptor.cursor.clone(),
            label: descriptor.label.clone(),
        }
    }
}

/// Pointer capture metadata for a viewport transform handle drag.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportTransformDragCapture {
    /// Stable handle identity captured at drag start.
    pub handle: ViewportTransformHandleId,
    /// Stable selection target identity captured at drag start.
    pub target: ViewportSelectionTargetId,
    /// Generic handle kind captured at drag start.
    pub kind: ViewportTransformHandleKind,
    /// Source target bounds in content coordinates at drag start.
    pub source_content_rect: Rect,
    /// Source target bounds in UI logical screen space at drag start.
    pub target_screen_rect: Rect,
    /// Handle hit rectangle in UI logical screen space at drag start.
    pub handle_screen_rect: Rect,
    /// Pointer position in UI logical screen space at drag start.
    pub pointer_origin_screen: Point,
    /// Pointer position in content coordinates at drag start, when conversion is possible.
    pub pointer_origin_content: Option<Point>,
}

impl ViewportTransformDragCapture {
    /// Creates pointer capture metadata from a handle hit.
    #[must_use]
    pub fn from_hit(hit: &ViewportTransformHandleHit) -> Self {
        Self {
            handle: hit.handle,
            target: hit.target,
            kind: hit.kind,
            source_content_rect: hit.source_content_rect,
            target_screen_rect: hit.target_screen_rect,
            handle_screen_rect: hit.handle_screen_rect,
            pointer_origin_screen: hit.point,
            pointer_origin_content: hit.content_point,
        }
    }
}

/// Status for a viewport transform drag request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportTransformDragStatus {
    /// The target is present and can receive transform request metadata.
    Active,
    /// The captured target ID no longer appears in the current target descriptors.
    StaleTarget,
    /// The target is disabled, unavailable, read-only, unselected, or no longer exposes the handle.
    UnavailableTarget,
    /// The current pointer position was not finite and was replaced by the capture origin.
    InvalidPointer,
    /// The viewport could not convert screen deltas into content deltas.
    InvalidScale,
}

/// Data-only transform drag update request metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportTransformDragRequest {
    /// Drag request status.
    pub status: ViewportTransformDragStatus,
    /// Stable handle identity captured at drag start.
    pub handle: ViewportTransformHandleId,
    /// Stable selection target identity captured at drag start.
    pub target: ViewportSelectionTargetId,
    /// Generic handle kind captured at drag start.
    pub kind: ViewportTransformHandleKind,
    /// Source target bounds in content coordinates at drag start.
    pub source_content_rect: Rect,
    /// Current target bounds in content coordinates, when the target is present.
    pub current_content_rect: Option<Rect>,
    /// Pointer position in UI logical screen space at drag start.
    pub pointer_origin_screen: Point,
    /// Current pointer position in UI logical screen space.
    pub pointer_current_screen: Point,
    /// Pointer position in content coordinates at drag start, when conversion is possible.
    pub pointer_origin_content: Option<Point>,
    /// Current pointer position in content coordinates, when conversion is possible.
    pub pointer_current_content: Option<Point>,
    /// Pointer delta in UI logical screen space from drag start.
    pub screen_delta: Vec2,
    /// Pointer delta in content coordinates from drag start.
    pub content_delta: Vec2,
}

impl ViewportTransformDragRequest {
    /// Creates drag update metadata from an existing pointer capture.
    #[must_use]
    pub fn update(
        surface: ViewportSurface,
        targets: &[ViewportSelectionTargetDescriptor],
        capture: &ViewportTransformDragCapture,
        pointer_current_screen: Point,
    ) -> Self {
        Self::update_at(
            surface,
            targets,
            capture,
            pointer_current_screen,
            ScaleFactor::ONE,
        )
    }

    /// Creates drag update metadata from an existing pointer capture for a viewport scale factor.
    #[must_use]
    pub fn update_at(
        surface: ViewportSurface,
        targets: &[ViewportSelectionTargetDescriptor],
        capture: &ViewportTransformDragCapture,
        pointer_current_screen: Point,
        scale_factor: ScaleFactor,
    ) -> Self {
        let pointer_is_valid = finite_point(pointer_current_screen).is_some();
        let pointer_current_screen = if pointer_is_valid {
            pointer_current_screen
        } else {
            capture.pointer_origin_screen
        };
        let screen_delta = Vec2::new(
            pointer_current_screen.x - capture.pointer_origin_screen.x,
            pointer_current_screen.y - capture.pointer_origin_screen.y,
        );
        let scale = finite_positive(surface.content_scale_at(scale_factor));
        let content_delta = scale.map_or(Vec2::ZERO, |scale| {
            Vec2::new(screen_delta.x / scale, screen_delta.y / scale)
        });
        let target = targets.iter().find(|target| target.id == capture.target);
        let current_content_rect =
            target.and_then(|target| finite_positive_rect(target.content_rect));
        let status = if !pointer_is_valid {
            ViewportTransformDragStatus::InvalidPointer
        } else if scale.is_none() {
            ViewportTransformDragStatus::InvalidScale
        } else {
            match target {
                None => ViewportTransformDragStatus::StaleTarget,
                Some(target)
                    if current_content_rect.is_some()
                        && target.can_request_transform()
                        && target.handles.contains(capture.kind) =>
                {
                    ViewportTransformDragStatus::Active
                }
                Some(_) => ViewportTransformDragStatus::UnavailableTarget,
            }
        };

        Self {
            status,
            handle: capture.handle,
            target: capture.target,
            kind: capture.kind,
            source_content_rect: capture.source_content_rect,
            current_content_rect,
            pointer_origin_screen: capture.pointer_origin_screen,
            pointer_current_screen,
            pointer_origin_content: capture.pointer_origin_content,
            pointer_current_content: surface
                .screen_to_content_at(pointer_current_screen, scale_factor),
            screen_delta,
            content_delta,
        }
    }

    /// Returns true when this request is deterministic no-op/error metadata.
    #[must_use]
    pub const fn is_noop(&self) -> bool {
        !matches!(self.status, ViewportTransformDragStatus::Active)
    }
}

/// Viewport overlay target category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportOverlayKind {
    /// Application-owned texture surface under the UI.
    TextureSurface,
    /// Source content bounds transformed by the viewport pan/zoom.
    ContentBounds,
    /// Guide-like overlay region.
    Guide,
    /// Tool-owned generic overlay region.
    ToolRegion,
}

/// Coordinate space used by a viewport overlay target rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportOverlaySpace {
    /// Rectangle is already in UI logical screen space.
    Screen,
    /// Rectangle is local to the viewport bounds.
    Viewport,
    /// Rectangle is in content coordinates and must be transformed by the viewport surface.
    Content,
}

/// Data-only viewport overlay hit target descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportOverlayDescriptor {
    /// Stable overlay identity.
    pub id: ViewportOverlayId,
    /// Overlay target category.
    pub kind: ViewportOverlayKind,
    /// Overlay target rectangle in `space`.
    pub rect: Rect,
    /// Coordinate space used by `rect`.
    pub space: ViewportOverlaySpace,
    /// Explicit hit-test priority. Higher priority wins.
    pub priority: i32,
    /// Optional owning tool identity.
    pub tool: Option<ViewportToolId>,
    /// Optional cursor request metadata.
    pub cursor: Option<ViewportCursorMetadata>,
    /// Whether this overlay can emit interaction requests.
    pub enabled: bool,
    /// Whether this overlay is available in the current app context.
    pub available: bool,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportOverlayDescriptor {
    /// Creates an enabled, available overlay target descriptor.
    #[must_use]
    pub fn new(
        id: ViewportOverlayId,
        kind: ViewportOverlayKind,
        rect: Rect,
        space: ViewportOverlaySpace,
    ) -> Self {
        Self {
            id,
            kind,
            rect,
            space,
            priority: default_viewport_overlay_priority(kind),
            tool: None,
            cursor: None,
            enabled: true,
            available: true,
            label: None,
        }
    }

    /// Creates a texture-surface target from the viewport source bounds.
    #[must_use]
    pub fn texture_surface(id: ViewportOverlayId, surface: ViewportSurface) -> Self {
        Self::content_bounds_with_kind(id, surface, ViewportOverlayKind::TextureSurface)
    }

    /// Creates a content-bounds target from the viewport source bounds.
    #[must_use]
    pub fn content_bounds(id: ViewportOverlayId, surface: ViewportSurface) -> Self {
        Self::content_bounds_with_kind(id, surface, ViewportOverlayKind::ContentBounds)
    }

    fn content_bounds_with_kind(
        id: ViewportOverlayId,
        surface: ViewportSurface,
        kind: ViewportOverlayKind,
    ) -> Self {
        let rect = surface.effective_source_size().map_or(Rect::ZERO, |size| {
            Rect::new(0.0, 0.0, size.width, size.height)
        });
        Self::new(id, kind, rect, ViewportOverlaySpace::Content)
    }

    /// Sets explicit hit-test priority. Higher priority wins.
    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Attaches an owning tool identity.
    #[must_use]
    pub fn with_tool(mut self, tool: ViewportToolId) -> Self {
        self.tool = Some(tool);
        self
    }

    /// Adds cursor request metadata.
    #[must_use]
    pub fn with_cursor(mut self, cursor: ViewportCursorMetadata) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Adds an accessible/debug label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Marks the overlay target as enabled or disabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Marks the overlay target as available or unavailable in the current app context.
    #[must_use]
    pub fn available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }

    /// Returns true when this overlay can emit interaction requests.
    #[must_use]
    pub const fn can_interact(&self) -> bool {
        self.enabled && self.available
    }

    /// Returns the transformed UI-space rectangle for hit testing.
    #[must_use]
    pub fn screen_rect(&self, surface: ViewportSurface, scale_factor: ScaleFactor) -> Option<Rect> {
        let rect = finite_positive_rect(self.rect)?;
        match self.space {
            ViewportOverlaySpace::Screen => Some(rect),
            ViewportOverlaySpace::Viewport => {
                let bounds = surface.effective_bounds();
                finite_positive_rect(Rect::new(
                    bounds.x + rect.x,
                    bounds.y + rect.y,
                    rect.width,
                    rect.height,
                ))
            }
            ViewportOverlaySpace::Content => surface.content_rect_to_screen_at(rect, scale_factor),
        }
        .and_then(finite_positive_rect)
    }
}

/// Data-only result of viewport overlay hit testing.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportOverlayHit {
    /// Stable overlay identity.
    pub overlay: ViewportOverlayId,
    /// Overlay target category.
    pub kind: ViewportOverlayKind,
    /// Hit rectangle in UI logical screen space.
    pub rect: Rect,
    /// Hit point in UI logical screen space.
    pub point: Point,
    /// Hit point transformed into content coordinates when possible.
    pub content_point: Option<Point>,
    /// Winning explicit hit priority.
    pub priority: i32,
    /// Optional owning tool identity.
    pub tool: Option<ViewportToolId>,
    /// Cursor request metadata associated with the hit target.
    pub cursor: Option<ViewportCursorMetadata>,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportOverlayHit {
    fn from_descriptor(
        descriptor: &ViewportOverlayDescriptor,
        rect: Rect,
        point: Point,
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            overlay: descriptor.id,
            kind: descriptor.kind,
            rect,
            point,
            content_point: surface.screen_to_content_at(point, scale_factor),
            priority: descriptor.priority,
            tool: descriptor.tool,
            cursor: descriptor.cursor.clone(),
            label: descriptor.label.clone(),
        }
    }
}

/// Source that determined a viewport cursor request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportCursorRequestSource {
    /// Active captured transform handle takes priority over hover.
    ActiveHandle,
    /// Hovered transform handle.
    HoveredHandle,
    /// Hovered overlay target.
    HoveredOverlay,
    /// Active viewport tool fallback.
    ActiveTool,
}

/// Data-only viewport cursor request selected from tool and hit metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportCursorRequest {
    /// Stable viewport widget identity.
    pub viewport: WidgetId,
    /// Winning cursor metadata.
    pub cursor: ViewportCursorMetadata,
    /// Source that selected the request.
    pub source: ViewportCursorRequestSource,
    /// Optional owning tool identity.
    pub tool: Option<ViewportToolId>,
    /// Optional overlay target identity.
    pub overlay: Option<ViewportOverlayId>,
    /// Optional transform handle identity.
    pub handle: Option<ViewportTransformHandleId>,
    /// Optional selection target identity.
    pub target: Option<ViewportSelectionTargetId>,
}

/// Resolves cursor request metadata with deterministic viewport priority.
///
/// Priority is active captured handle, hovered handle, hovered overlay, then
/// active tool. The result is data-only so platform adapters can translate it
/// without widgets calling native APIs.
#[must_use]
pub fn viewport_cursor_request(
    viewport: WidgetId,
    active_handle: Option<&ViewportTransformDragCapture>,
    hovered_handle: Option<&ViewportTransformHandleHit>,
    hovered_overlay: Option<&ViewportOverlayHit>,
    active_tool: Option<&ViewportToolDescriptor>,
) -> Option<ViewportCursorRequest> {
    if let Some(active_handle) = active_handle {
        return Some(ViewportCursorRequest {
            viewport,
            cursor: ViewportCursorMetadata::new(active_handle.kind.cursor_shape()),
            source: ViewportCursorRequestSource::ActiveHandle,
            tool: None,
            overlay: None,
            handle: Some(active_handle.handle),
            target: Some(active_handle.target),
        });
    }

    if let Some(hovered_handle) = hovered_handle {
        return Some(ViewportCursorRequest {
            viewport,
            cursor: hovered_handle.cursor.clone(),
            source: ViewportCursorRequestSource::HoveredHandle,
            tool: None,
            overlay: None,
            handle: Some(hovered_handle.handle),
            target: Some(hovered_handle.target),
        });
    }

    if let Some((hovered_overlay, cursor)) =
        hovered_overlay.and_then(|hit| hit.cursor.as_ref().map(|cursor| (hit, cursor)))
    {
        return Some(ViewportCursorRequest {
            viewport,
            cursor: cursor.clone(),
            source: ViewportCursorRequestSource::HoveredOverlay,
            tool: hovered_overlay.tool,
            overlay: Some(hovered_overlay.overlay),
            handle: None,
            target: None,
        });
    }

    active_tool.and_then(|tool| {
        tool.cursor_request()
            .cloned()
            .map(|cursor| ViewportCursorRequest {
                viewport,
                cursor,
                source: ViewportCursorRequestSource::ActiveTool,
                tool: Some(tool.id),
                overlay: None,
                handle: None,
                target: None,
            })
    })
}

/// Resolves a UI-space point to the highest-priority viewport overlay target.
///
/// Disabled or unavailable overlays are skipped. Higher `priority` wins. When
/// priorities tie, the lower stable `ViewportOverlayId` wins so the result does
/// not depend on primitive emission order or descriptor ordering.
#[must_use]
pub fn hit_test_viewport_overlays(
    surface: ViewportSurface,
    overlays: &[ViewportOverlayDescriptor],
    point: Point,
) -> Option<ViewportOverlayHit> {
    hit_test_viewport_overlays_at(surface, overlays, point, ScaleFactor::ONE)
}

/// Resolves a UI-space point to the highest-priority viewport overlay target
/// for a viewport scale factor.
#[must_use]
pub fn hit_test_viewport_overlays_at(
    surface: ViewportSurface,
    overlays: &[ViewportOverlayDescriptor],
    point: Point,
    scale_factor: ScaleFactor,
) -> Option<ViewportOverlayHit> {
    let point = finite_point(point)?;
    overlays
        .iter()
        .filter(|overlay| overlay.can_interact())
        .filter_map(|overlay| {
            let rect = overlay.screen_rect(surface, scale_factor)?;
            rect.contains_point(point).then(|| {
                ViewportOverlayHit::from_descriptor(overlay, rect, point, surface, scale_factor)
            })
        })
        .max_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| right.overlay.cmp(&left.overlay))
        })
}
