#[allow(clippy::wildcard_imports)]
use super::*;

/// Data-only pan/zoom HUD descriptor supplied by the application.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportPanZoomHudDescriptor {
    /// Stable HUD semantic identity.
    pub id: WidgetId,
    /// HUD accessible/debug label.
    pub label: String,
    /// Optional focused selection target.
    pub focused_target: Option<ViewportSelectionTargetId>,
    /// Current selected target IDs.
    pub selected_targets: Vec<ViewportSelectionTargetId>,
}

impl ViewportPanZoomHudDescriptor {
    /// Creates a viewport pan/zoom HUD descriptor.
    #[must_use]
    pub fn new(id: WidgetId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            focused_target: None,
            selected_targets: Vec::new(),
        }
    }

    /// Adds focused target metadata.
    #[must_use]
    pub const fn with_focused_target(mut self, target: ViewportSelectionTargetId) -> Self {
        self.focused_target = Some(target);
        self
    }

    /// Adds selected target metadata.
    #[must_use]
    pub fn with_selected_targets(mut self, targets: &[ViewportSelectionTargetId]) -> Self {
        self.selected_targets.extend_from_slice(targets);
        self
    }

    /// Resolves the HUD descriptor against a viewport surface.
    #[must_use]
    pub fn resolve(&self, surface: ViewportSurface) -> ViewportPanZoomHud {
        let mut selected_targets = self.selected_targets.clone();
        selected_targets.sort();
        selected_targets.dedup();
        ViewportPanZoomHud {
            id: self.id,
            label: self.label.clone(),
            fit: surface.pan_zoom.fit,
            zoom: finite_positive(surface.pan_zoom.zoom).unwrap_or(1.0),
            effective_content_scale: finite_or_zero(surface.content_scale()),
            pan: Vec2::new(
                finite_or_zero(surface.pan_zoom.pan.x),
                finite_or_zero(surface.pan_zoom.pan.y),
            ),
            content_size: surface.effective_source_size().unwrap_or(Size::ZERO),
            focused_target: self.focused_target,
            selected_targets,
        }
    }
}

/// Pan/zoom HUD metadata resolved from a viewport surface.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportPanZoomHud {
    /// Stable HUD semantic identity.
    pub id: WidgetId,
    /// HUD accessible/debug label.
    pub label: String,
    /// Current fit mode.
    pub fit: ViewportFit,
    /// Current custom zoom value.
    pub zoom: f32,
    /// Effective content-to-screen scale after fit, zoom, and DPI policy.
    pub effective_content_scale: f32,
    /// Sanitized pan offset in logical screen units.
    pub pan: Vec2,
    /// Sanitized source content size.
    pub content_size: Size,
    /// Optional focused selection target.
    pub focused_target: Option<ViewportSelectionTargetId>,
    /// Sorted, deduplicated selected target IDs.
    pub selected_targets: Vec<ViewportSelectionTargetId>,
}

impl ViewportPanZoomHud {
    /// Builds a stable, generic text value for HUD semantics or debug surfaces.
    #[must_use]
    pub fn value_text(&self) -> String {
        format!(
            "{:?} zoom {:.3}, scale {:.3}, pan {:.3},{:.3}, content {:.3}x{:.3}, selected {}",
            self.fit,
            self.zoom,
            self.effective_content_scale,
            self.pan.x,
            self.pan.y,
            self.content_size.width,
            self.content_size.height,
            self.selected_targets.len()
        )
    }

    /// Builds backend-neutral semantic metadata for this HUD.
    #[must_use]
    pub fn semantics(&self, bounds: Rect) -> SemanticNode {
        let mut node = SemanticNode::new(
            self.id,
            SemanticRole::Custom("viewport-pan-zoom-hud".to_owned()),
            sanitize_rect(bounds),
        )
        .with_label(self.label.clone());
        node.state.value = Some(SemanticValue::Text(self.value_text()));
        node
    }
}

/// Resolves selected target outlines into UI logical screen space.
#[must_use]
pub fn viewport_selection_outlines(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
) -> Vec<ViewportSelectionOutlineDescriptor> {
    viewport_selection_outlines_at(surface, targets, ScaleFactor::ONE)
}

/// Resolves selected target outlines into UI logical screen space for a viewport scale factor.
#[must_use]
pub fn viewport_selection_outlines_at(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
    scale_factor: ScaleFactor,
) -> Vec<ViewportSelectionOutlineDescriptor> {
    let mut outlines = targets
        .iter()
        .filter_map(|target| {
            ViewportSelectionOutlineDescriptor::from_target(target, surface, scale_factor)
        })
        .collect::<Vec<_>>();
    outlines.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.target.cmp(&right.target))
    });
    outlines
}

/// Resolves selected target transform handles into UI logical screen space.
#[must_use]
pub fn viewport_transform_handles(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
) -> Vec<ViewportTransformHandleDescriptor> {
    viewport_transform_handles_at(surface, targets, ScaleFactor::ONE)
}

/// Resolves selected target transform handles into UI logical screen space for a scale factor.
#[must_use]
pub fn viewport_transform_handles_at(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
    scale_factor: ScaleFactor,
) -> Vec<ViewportTransformHandleDescriptor> {
    let mut handles = targets
        .iter()
        .filter(|target| target.can_request_transform())
        .filter_map(|target| {
            let source_content_rect = finite_positive_rect(target.content_rect)?;
            let target_screen_rect = target.screen_rect(surface, scale_factor)?;
            Some((target, source_content_rect, target_screen_rect))
        })
        .flat_map(|(target, source_content_rect, target_screen_rect)| {
            target.handles.kinds().filter_map(move |kind| {
                let handle_screen_rect = transform_handle_rect(target, target_screen_rect, kind)?;
                Some(ViewportTransformHandleDescriptor {
                    id: ViewportTransformHandleId::new(target.id, kind),
                    target: target.id,
                    kind,
                    source_content_rect,
                    target_screen_rect,
                    handle_screen_rect,
                    target_priority: target.priority,
                    handle_priority: kind.hit_priority(),
                    cursor: ViewportCursorMetadata::new(kind.cursor_shape()),
                    label: target.label.clone(),
                })
            })
        })
        .collect::<Vec<_>>();
    handles.sort_by(|left, right| {
        left.target_priority
            .cmp(&right.target_priority)
            .then_with(|| left.handle_priority.cmp(&right.handle_priority))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.kind.cmp(&right.kind))
    });
    handles
}

/// Resolves a UI-space point to the highest-priority viewport transform handle.
///
/// Disabled, unavailable, read-only, and unselected targets are skipped.
/// Higher target priority is treated as topmost. Within one target, more
/// specific handles win over broad move regions. Stable target and handle IDs
/// break remaining ties so descriptor order does not affect the result.
#[must_use]
pub fn hit_test_viewport_transform_handles(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
    point: Point,
) -> Option<ViewportTransformHandleHit> {
    hit_test_viewport_transform_handles_at(surface, targets, point, ScaleFactor::ONE)
}

/// Resolves a UI-space point to the highest-priority viewport transform handle
/// for a viewport scale factor.
#[must_use]
pub fn hit_test_viewport_transform_handles_at(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
    point: Point,
    scale_factor: ScaleFactor,
) -> Option<ViewportTransformHandleHit> {
    let point = finite_point(point)?;
    viewport_transform_handles_at(surface, targets, scale_factor)
        .into_iter()
        .filter(|handle| handle.handle_screen_rect.contains_point(point))
        .map(|handle| {
            ViewportTransformHandleHit::from_descriptor(&handle, point, surface, scale_factor)
        })
        .max_by(|left, right| {
            left.target_priority
                .cmp(&right.target_priority)
                .then_with(|| left.handle_priority.cmp(&right.handle_priority))
                .then_with(|| right.target.cmp(&left.target))
                .then_with(|| right.kind.cmp(&left.kind))
        })
}

pub(crate) fn transform_handle_rect(
    target: &ViewportSelectionTargetDescriptor,
    target_screen_rect: Rect,
    kind: ViewportTransformHandleKind,
) -> Option<Rect> {
    if kind == ViewportTransformHandleKind::Move {
        return finite_positive_rect(target_screen_rect);
    }

    let size = target.effective_handle_size();
    let half = size * 0.5;
    let center =
        transform_handle_center(target_screen_rect, target.effective_rotate_offset(), kind);
    finite_positive_rect(Rect::new(center.x - half, center.y - half, size, size))
}

pub(crate) fn transform_handle_center(
    rect: Rect,
    rotate_offset: f32,
    kind: ViewportTransformHandleKind,
) -> Point {
    let center = rect.center();
    match kind {
        ViewportTransformHandleKind::Move | ViewportTransformHandleKind::Pivot => center,
        ViewportTransformHandleKind::ResizeTopLeft => Point::new(rect.x, rect.y),
        ViewportTransformHandleKind::ResizeTop => Point::new(center.x, rect.y),
        ViewportTransformHandleKind::ResizeTopRight => Point::new(rect.max_x(), rect.y),
        ViewportTransformHandleKind::ResizeRight => Point::new(rect.max_x(), center.y),
        ViewportTransformHandleKind::ResizeBottomRight => Point::new(rect.max_x(), rect.max_y()),
        ViewportTransformHandleKind::ResizeBottom => Point::new(center.x, rect.max_y()),
        ViewportTransformHandleKind::ResizeBottomLeft => Point::new(rect.x, rect.max_y()),
        ViewportTransformHandleKind::ResizeLeft => Point::new(rect.x, center.y),
        ViewportTransformHandleKind::Rotate => Point::new(center.x, rect.y - rotate_offset),
    }
}
