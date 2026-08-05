//! Headless story frame composition.
//!
//! Composes one story variant into a deterministic [`FrameOutput`] plus the
//! renderer resources for its shaped text. No window, no GPU: the same
//! immediate-mode path the real shell drives, fed by a default input
//! snapshot.

use stern::UiState;
use stern::core::{
    FrameContext, FrameOutput, PhysicalSize, Rect, ScaleFactor, Size, TimeInfo, UiInput,
    ViewportInfo, default_dark_theme,
};
use stern::render::RenderResources;

use crate::story::{Story, StoryVariant};

/// One composed story frame ready for rasterization.
pub struct ComposedFrame {
    /// Deterministic frame output (primitives, semantics, warnings).
    pub output: FrameOutput,
    /// Renderer resources holding the frame's shaped text layouts.
    pub resources: RenderResources,
}

/// Physical pixel size for a logical size at a scale factor.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn physical_size(logical: Size, scale: f32) -> PhysicalSize {
    PhysicalSize::new(
        (logical.width * scale).round().max(1.0) as u32,
        (logical.height * scale).round().max(1.0) as u32,
    )
}

/// Composes `story` at `variant` into a frame using fresh retained state.
#[must_use]
pub fn compose_story_frame(story: &Story, variant: StoryVariant) -> ComposedFrame {
    let mut state = UiState::new();
    let frame = compose_with_state(&mut state, story, variant);
    let resources = state.text_render_resources();
    ComposedFrame {
        output: frame,
        resources,
    }
}

/// Composes `story` at `variant` through caller-owned retained state.
#[must_use]
pub fn compose_with_state(
    state: &mut UiState,
    story: &Story,
    variant: StoryVariant,
) -> FrameOutput {
    let theme = default_dark_theme();
    let context = FrameContext::new(
        ViewportInfo::new(
            variant.logical,
            physical_size(variant.logical, variant.scale),
            ScaleFactor::new(f64::from(variant.scale)),
        ),
        UiInput::default(),
        TimeInfo::default(),
    );
    let mut ui = state.begin_frame(context, &theme);
    (story.compose)(
        &mut ui,
        Rect::new(0.0, 0.0, variant.logical.width, variant.logical.height),
    );
    ui.finish_output()
}
