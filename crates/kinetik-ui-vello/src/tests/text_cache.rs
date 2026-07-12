use super::common::text_layout_resource;
use crate::{
    RenderDiagnostic, RenderFrameInput, RenderResources, VelloRenderer,
    encode_forced_transient_text,
};
use kinetik_ui_core::{
    Brush, Color, PhysicalSize, Point, Primitive, ScaleFactor, Size, TextLayoutId, TextPrimitive,
    ViewportInfo,
};
use kinetik_ui_text::{TextLayoutKey, TextLayoutStore, TextStyle};
use vello::Scene;

const MAX_TEXT_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;

fn viewport(scale: f64) -> ViewportInfo {
    let (width, height) = match scale {
        1.25 => (125, 100),
        1.5 => (150, 120),
        1.75 => (175, 140),
        _ => panic!("unsupported fixture scale {scale}"),
    };
    ViewportInfo::new(
        Size::new(100.0, 80.0),
        PhysicalSize::new(width, height),
        ScaleFactor::new(scale),
    )
}

fn text(layout: Option<TextLayoutId>, value: impl Into<String>) -> Primitive {
    Primitive::Text(TextPrimitive {
        layout,
        origin: Point::new(4.3, 16.4),
        text: value.into(),
        family: "sans-serif".to_owned(),
        size: 13.0,
        line_height: 18.0,
        brush: Brush::Solid(Color::WHITE),
    })
}

#[test]
fn registered_text_never_populates_the_fallback_store() {
    let id = TextLayoutId::from_raw(48);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(id, "Registered"));
    let primitives = [text(Some(id), "conflicting fallback")];
    let mut renderer = VelloRenderer::new();

    for scale in [1.25, 1.5, 1.75] {
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(scale),
            primitives: &primitives,
            resources: &resources,
        });
        assert!(output.diagnostics.is_empty());
        assert_eq!(renderer.cached_text_layout_count(), 0);
        assert_eq!(renderer.cached_text_layout_payload_bytes(), 0);
    }
    assert_eq!(renderer.cached_text_layout_generation(), 3);
}

#[test]
fn layoutless_text_reuses_one_logical_entry_across_fractional_scales() {
    let resources = RenderResources::new();
    let primitives = [text(None, "Logical fallback")];
    let mut renderer = VelloRenderer::new();

    for scale in [1.25, 1.5, 1.75] {
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(scale),
            primitives: &primitives,
            resources: &resources,
        });
        assert!(output.diagnostics.is_empty());
        assert_eq!(renderer.cached_text_layout_count(), 1);
    }
    assert_eq!(renderer.cached_text_layout_generation(), 3);
    assert!(renderer.cached_text_layout_payload_bytes() > 0);
    assert!(renderer.cached_text_layout_payload_bytes() <= MAX_TEXT_PAYLOAD_BYTES);
}

#[test]
fn every_submission_advances_fallback_generation_exactly_once() {
    let registered = TextLayoutId::from_raw(49);
    let missing = TextLayoutId::from_raw(50);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(registered, "Registered"));
    let mut renderer = VelloRenderer::new();

    let cases = [
        Vec::new(),
        vec![text(Some(registered), "ignored")],
        vec![text(None, "Fallback")],
        vec![text(Some(missing), "Fallback")],
    ];
    for (index, primitives) in cases.iter().enumerate() {
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(1.25),
            primitives,
            resources: &resources,
        });
        assert_eq!(
            renderer.cached_text_layout_generation(),
            u64::try_from(index + 1).expect("generation fits")
        );
        if index == 3 {
            assert_eq!(
                output.diagnostics,
                vec![RenderDiagnostic::MissingTextLayout(missing)]
            );
        } else {
            assert!(output.diagnostics.is_empty());
        }
        if index >= 2 {
            assert!(!renderer.scene().encoding().resources.glyph_runs.is_empty());
            assert!(!renderer.scene().encoding().resources.glyphs.is_empty());
        }
    }
    assert_eq!(renderer.cached_text_layout_count(), 1);
}

#[test]
fn fallback_survives_120_idle_generations_and_expires_on_121() {
    let resources = RenderResources::new();
    let fallback = [text(None, "Idle fallback")];
    let mut renderer = VelloRenderer::new();
    let _ = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.25),
        primitives: &fallback,
        resources: &resources,
    });

    for _ in 0..120 {
        let _ = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(1.25),
            primitives: &[],
            resources: &resources,
        });
    }
    assert_eq!(renderer.cached_text_layout_generation(), 121);
    assert_eq!(renderer.cached_text_layout_count(), 1);

    let _ = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.25),
        primitives: &[],
        resources: &resources,
    });
    assert_eq!(renderer.cached_text_layout_generation(), 122);
    assert_eq!(renderer.cached_text_layout_count(), 0);
    assert_eq!(renderer.cached_text_layout_payload_bytes(), 0);
}

#[test]
fn one_thousand_dynamic_fallback_labels_remain_generation_and_byte_bounded() {
    let resources = RenderResources::new();
    let mut renderer = VelloRenderer::new();

    for index in 0..1_000 {
        let primitives = [text(None, format!("dynamic fallback {index}"))];
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(1.75),
            primitives: &primitives,
            resources: &resources,
        });
        assert!(output.diagnostics.is_empty());
        assert!(renderer.cached_text_layout_count() <= 121);
        assert!(renderer.cached_text_layout_payload_bytes() <= MAX_TEXT_PAYLOAD_BYTES);
    }
    assert_eq!(renderer.cached_text_layout_generation(), 1_000);
}

#[test]
fn forced_transient_branch_paints_without_retaining_the_small_fixture() {
    let mut layouts = TextLayoutStore::new();
    let mut scene = Scene::new();
    let key = TextLayoutKey::new(
        "transient",
        TextStyle::new("sans-serif", 13.0, 18.0),
        0.0,
        false,
    );

    let layout = encode_forced_transient_text(&mut scene, &mut layouts, &key);

    assert!(!layout.is_empty());
    assert!(!scene.encoding().resources.glyph_runs.is_empty());
    assert!(!scene.encoding().resources.glyphs.is_empty());
    assert!(layouts.is_empty());
    assert_eq!(layouts.retained_payload_bytes(), 0);
}
