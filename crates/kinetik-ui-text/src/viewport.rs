use kinetik_ui_core::{Rect, Size, Vec2};

/// The axis along which a text field viewport may move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextViewportMode {
    /// A single-line field that scrolls horizontally.
    SingleLine,
    /// A wrapped multi-line field that scrolls vertically.
    WrappedMultiLine,
}

/// Sanitized geometry for a text field's scrollable content viewport.
///
/// Offsets are positive content displacements. Consumers therefore paint
/// content at the negated offset returned by [`Self::offset`]. The helper is
/// immutable and owns no retained widget or platform state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextViewport {
    mode: TextViewportMode,
    viewport: Size,
    content: Size,
    offset: Vec2,
    max_offset: Vec2,
}

impl TextViewport {
    /// Creates a viewport after sanitizing extents and clamping the offset to
    /// the axis permitted by `mode`.
    #[must_use]
    pub fn new(mode: TextViewportMode, viewport: Size, content: Size, offset: Vec2) -> Self {
        let viewport = sanitize_size(viewport);
        let content = sanitize_size(content);
        let max_offset = match mode {
            TextViewportMode::SingleLine => {
                Vec2::new((content.width - viewport.width).max(0.0), 0.0)
            }
            TextViewportMode::WrappedMultiLine => {
                Vec2::new(0.0, (content.height - viewport.height).max(0.0))
            }
        };
        let offset = clamp_offset(mode, sanitize_vec(offset), max_offset);

        Self {
            mode,
            viewport,
            content,
            offset,
            max_offset,
        }
    }

    /// Returns the permitted scrolling axis.
    #[must_use]
    pub const fn mode(&self) -> TextViewportMode {
        self.mode
    }

    /// Returns the sanitized visible viewport size.
    #[must_use]
    pub const fn viewport_size(&self) -> Size {
        self.viewport
    }

    /// Returns the sanitized unscrolled content size.
    #[must_use]
    pub const fn content_size(&self) -> Size {
        self.content
    }

    /// Returns the clamped positive content displacement.
    #[must_use]
    pub const fn offset(&self) -> Vec2 {
        self.offset
    }

    /// Returns the greatest permitted displacement on the scrolling axis.
    #[must_use]
    pub const fn max_offset(&self) -> Vec2 {
        self.max_offset
    }

    /// Returns the offset produced by applying `delta` to the current offset.
    ///
    /// Non-finite delta components are ignored. The cross-axis component is
    /// always zero.
    #[must_use]
    pub fn scroll_by(&self, delta: Vec2) -> Vec2 {
        let delta = sanitize_vec(delta);
        let candidate = Vec2::new(self.offset.x + delta.x, self.offset.y + delta.y);
        clamp_offset(self.mode, candidate, self.max_offset)
    }

    /// Returns the nearest offset that fully reveals `target` when possible.
    ///
    /// The target uses unscrolled content coordinates. Invalid targets leave
    /// the current offset unchanged. Targets larger than the viewport on the
    /// scrolling axis align their leading edge before clamping.
    #[must_use]
    pub fn reveal(&self, target: Rect) -> Vec2 {
        if !valid_target(target) {
            return self.offset;
        }

        match self.mode {
            TextViewportMode::SingleLine => Vec2::new(
                reveal_axis(
                    self.offset.x,
                    self.viewport.width,
                    self.max_offset.x,
                    target.x,
                    target.width,
                ),
                0.0,
            ),
            TextViewportMode::WrappedMultiLine => Vec2::new(
                0.0,
                reveal_axis(
                    self.offset.y,
                    self.viewport.height,
                    self.max_offset.y,
                    target.y,
                    target.height,
                ),
            ),
        }
    }
}

fn sanitize_size(size: Size) -> Size {
    Size::new(sanitize_extent(size.width), sanitize_extent(size.height))
}

fn sanitize_extent(extent: f32) -> f32 {
    if extent.is_finite() && extent >= 0.0 {
        extent
    } else {
        0.0
    }
}

fn sanitize_vec(vector: Vec2) -> Vec2 {
    Vec2::new(
        sanitize_offset_component(vector.x),
        sanitize_offset_component(vector.y),
    )
}

fn sanitize_offset_component(component: f32) -> f32 {
    if component.is_finite() {
        component
    } else {
        0.0
    }
}

fn clamp_offset(mode: TextViewportMode, offset: Vec2, max_offset: Vec2) -> Vec2 {
    match mode {
        TextViewportMode::SingleLine => Vec2::new(offset.x.clamp(0.0, max_offset.x), 0.0),
        TextViewportMode::WrappedMultiLine => Vec2::new(0.0, offset.y.clamp(0.0, max_offset.y)),
    }
}

fn valid_target(target: Rect) -> bool {
    target.x.is_finite()
        && target.y.is_finite()
        && target.width.is_finite()
        && target.height.is_finite()
        && target.width >= 0.0
        && target.height >= 0.0
}

fn reveal_axis(
    offset: f32,
    viewport_extent: f32,
    max_offset: f32,
    target_start: f32,
    target_extent: f32,
) -> f32 {
    if target_extent > viewport_extent {
        return target_start.clamp(0.0, max_offset);
    }

    let target_end = target_start + target_extent;
    let viewport_end = offset + viewport_extent;

    if target_start < offset {
        target_start.clamp(0.0, max_offset)
    } else if target_end > viewport_end {
        (target_end - viewport_extent).clamp(0.0, max_offset)
    } else {
        offset
    }
}
