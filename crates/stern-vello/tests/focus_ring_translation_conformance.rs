//! Public Stern/Vello focus-ring translation conformance.

#![allow(clippy::float_cmp)]

use stern_core::{
    CornerRadius, PhysicalSize, Primitive, Rect, ScaleFactor, Size, ViewportInfo,
    default_dark_theme,
};
use stern_vello::{
    RenderCommandKind, RenderFrameInput, RenderResources, VelloRenderer, translate_primitives,
};

#[test]
fn nested_focus_fills_preserve_order_through_vello_at_release_scales() {
    let recipe = default_dark_theme()
        .focus_ring(true)
        .expect("visible focus ring");
    let rect = Rect::new(10.0, 20.0, 20.0, 20.0);
    let primitives = recipe.primitives(rect, CornerRadius::all(4.0));

    let [Primitive::Rect(outer), Primitive::Rect(inner)] = &primitives else {
        panic!("focus recipe must emit exactly two rectangle primitives");
    };
    assert_eq!(outer.fill, Some(recipe.primary.brush));
    assert_eq!(outer.stroke, None);
    assert_eq!(inner.fill, Some(recipe.separator.brush));
    assert_eq!(inner.stroke, None);

    let resources = RenderResources::new();
    let translation = translate_primitives(&primitives, &resources);
    assert!(translation.diagnostics.is_empty());
    assert_eq!(translation.commands.len(), 2);

    for (command, expected) in translation.commands.iter().zip([outer, inner]) {
        let RenderCommandKind::Rect {
            rect,
            fill,
            stroke,
            radius,
        } = &command.kind
        else {
            panic!("focus contour must translate to a rectangle command");
        };
        assert_eq!(*rect, expected.rect);
        assert_eq!(*fill, expected.fill);
        assert_eq!(*stroke, None);
        assert_eq!(*radius, expected.radius);
    }

    for (scale, physical_size) in [
        (1.0_f32, PhysicalSize::new(80, 80)),
        (1.25, PhysicalSize::new(100, 100)),
        (1.5, PhysicalSize::new(120, 120)),
        (2.0, PhysicalSize::new(160, 160)),
    ] {
        let mut renderer = VelloRenderer::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(80.0, 80.0),
                physical_size,
                ScaleFactor::new(f64::from(scale)),
            ),
            primitives: &primitives,
            resources: &resources,
        });

        assert_eq!(output.primitive_count, 2);
        assert!(
            output.diagnostics.is_empty(),
            "focus encoding diagnostics at {scale}x: {:?}",
            output.diagnostics
        );

        let encoding = renderer.scene().encoding();
        assert_eq!(encoding.n_paths, 2, "separate contours at {scale}x");
        assert_eq!(encoding.draw_tags.len(), 2, "separate fills at {scale}x");
        assert!(
            encoding.draw_tags.iter().all(|tag| tag.0 == 0x44),
            "both contours must remain solid-color fills at {scale}x"
        );
        assert_eq!(
            encoding.draw_data,
            vec![0xFFFF_B24D, 0xFF0B_0B0B],
            "outer-primary then inner-separator color order at {scale}x"
        );
        assert!(
            encoding.styles.iter().all(|style| {
                style.flags_and_miter_limit & 0x8000_0000 == 0 && style.line_width == 0.0
            }),
            "focus contours must not become stroked paths at {scale}x"
        );
        assert_eq!(encoding.transforms.len(), 1);
        assert_eq!(encoding.transforms[0].matrix, [scale, 0.0, 0.0, scale]);
    }
}
