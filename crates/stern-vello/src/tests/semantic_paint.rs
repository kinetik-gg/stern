use crate::{RenderFrameInput, RenderResources, VelloRenderer};
use stern_core::{
    FrameContext, FrameOutput, MouseButton, PhysicalSize, Point, Primitive, Rect, Response,
    ScaleFactor, SemanticRole, Size, TimeInfo, UiInput, UiInputEvent, UiMemory, Vec2, ViewportInfo,
    default_dark_theme,
};
use stern_widgets::ui::Ui;

const SOURCE_RECT: Rect = Rect::new(10.2, 20.2, 30.2, 18.2);
const DISCRIMINATING_POINT: Point = Point::new(40.6, 25.0);

fn canonical_click_input(pointer: Point) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerMoved {
        position: pointer,
        delta: Vec2::ZERO,
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(pointer),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(pointer),
    });
    input
}

fn compose_button(
    scale: f64,
    physical_size: PhysicalSize,
    pointer: Point,
) -> (Response, FrameOutput) {
    let viewport = ViewportInfo::new(
        Size::new(800.0, 600.0),
        physical_size,
        ScaleFactor::new(scale),
    );
    let context = FrameContext::new(
        viewport,
        canonical_click_input(pointer),
        TimeInfo::default(),
    );
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let response = ui.button("fractional-button", SOURCE_RECT, "Fractional", false);
    let output = ui.finish_output();
    assert!(output.warnings.is_empty());
    (response, output)
}

fn emitted_button_rect(output: &FrameOutput) -> Primitive {
    let mut rectangles = output
        .primitives
        .iter()
        .filter(|primitive| matches!(primitive, Primitive::Rect(_)));
    let primitive = rectangles
        .next()
        .expect("button background rectangle")
        .clone();
    assert!(rectangles.next().is_none(), "button emits one rectangle");
    let Primitive::Rect(rectangle) = &primitive else {
        unreachable!("filtered rectangle primitive")
    };
    assert_eq!(rectangle.rect, SOURCE_RECT);
    assert!(rectangle.fill.is_some());
    let stroke = rectangle.stroke.expect("button border recipe");
    assert!(stroke.width.is_finite() && stroke.width > 0.0);
    primitive
}

fn encoded_fill_device_bounds(
    primitive: &Primitive,
    scale: f64,
    physical_size: PhysicalSize,
) -> Rect {
    let viewport = ViewportInfo::new(
        Size::new(800.0, 600.0),
        physical_size,
        ScaleFactor::new(scale),
    );
    let resources = RenderResources::new();
    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: std::slice::from_ref(primitive),
        resources: &resources,
    });
    assert!(output.diagnostics.is_empty());

    let encoding = renderer.scene().encoding();
    assert_eq!(encoding.n_paths, 2);
    assert_eq!(encoding.transforms.len(), 1);
    let first_path_end = encoding
        .path_tags
        .iter()
        .position(|tag| tag.0 == 0x10)
        .expect("encoded fill path marker");
    let first_path_point_count = encoding.path_tags[..first_path_end]
        .iter()
        .filter(|tag| tag.is_path_segment())
        .map(|tag| {
            assert!(tag.is_f32(), "button fill path uses f32 coordinates");
            usize::from(tag.path_segment_type().0) + usize::from(tag.is_subpath_end())
        })
        .sum::<usize>();
    assert!(first_path_point_count > 0);
    let first_path_word_count = first_path_point_count * 2;
    let first_path_data = encoding
        .path_data
        .get(..first_path_word_count)
        .expect("complete encoded fill path data");
    let transform = encoding.transforms[0];
    let mut min = Point::new(f32::INFINITY, f32::INFINITY);
    let mut max = Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut words = first_path_data.chunks_exact(2);
    for point_words in &mut words {
        let x = f32::from_bits(point_words[0]);
        let y = f32::from_bits(point_words[1]);
        let point = Point::new(
            transform.matrix[0]
                .mul_add(x, transform.matrix[2].mul_add(y, transform.translation[0])),
            transform.matrix[1]
                .mul_add(x, transform.matrix[3].mul_add(y, transform.translation[1])),
        );
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    assert!(words.remainder().is_empty());
    assert!(min.x.is_finite() && min.y.is_finite());
    assert!(max.x.is_finite() && max.y.is_finite());
    Rect::from_min_max(min, max)
}

fn assert_rect_close(actual: Rect, expected: Rect) {
    for (actual, expected) in [
        (actual.x, expected.x),
        (actual.y, expected.y),
        (actual.width, expected.width),
        (actual.height, expected.height),
    ] {
        assert!(
            (actual - expected).abs() < 0.000_01,
            "expected {actual} to equal {expected}"
        );
    }
}

#[test]
fn composed_button_keeps_semantic_and_hit_geometry_logical_while_vello_snaps_paint() {
    for (scale, physical_size, expected_device_paint, paint_contains_discriminator) in [
        (
            1.0_f32,
            PhysicalSize::new(800, 600),
            Rect::new(10.0, 20.0, 30.0, 18.0),
            false,
        ),
        (
            1.25,
            PhysicalSize::new(1000, 750),
            Rect::new(13.0, 25.0, 38.0, 23.0),
            true,
        ),
        (
            1.5,
            PhysicalSize::new(1200, 900),
            Rect::new(15.0, 30.0, 46.0, 28.0),
            true,
        ),
        (
            2.0,
            PhysicalSize::new(1600, 1200),
            Rect::new(20.0, 40.0, 61.0, 37.0),
            false,
        ),
    ] {
        for (name, pointer, expected_hit) in [
            ("inside", Point::new(25.0, 25.0), true),
            ("outside", Point::new(10.19, 25.0), false),
            ("minimum boundary", Point::new(SOURCE_RECT.x, 25.0), true),
            (
                "maximum boundary",
                Point::new(SOURCE_RECT.max_x(), 25.0),
                false,
            ),
            ("logical/paint discriminator", DISCRIMINATING_POINT, false),
        ] {
            let (response, _) = compose_button(f64::from(scale), physical_size, pointer);
            assert_eq!(response.rect, SOURCE_RECT, "{name} rect at {scale}x");
            assert_eq!(
                response.state.hovered, expected_hit,
                "{name} hover at {scale}x"
            );
            assert_eq!(response.clicked, expected_hit, "{name} click at {scale}x");
        }

        let (response, output) =
            compose_button(f64::from(scale), physical_size, DISCRIMINATING_POINT);
        let semantic = output
            .semantics
            .get(response.id)
            .expect("button semantic node");
        assert_eq!(semantic.role, SemanticRole::Button);
        assert_eq!(semantic.bounds, SOURCE_RECT);

        let paint = emitted_button_rect(&output);
        let encoded_bounds = encoded_fill_device_bounds(&paint, f64::from(scale), physical_size);
        assert_rect_close(encoded_bounds, expected_device_paint);
        let physical_discriminator = Point::new(
            DISCRIMINATING_POINT.x * scale,
            DISCRIMINATING_POINT.y * scale,
        );
        assert_eq!(
            encoded_bounds.contains_point(physical_discriminator),
            paint_contains_discriminator,
            "encoded paint discriminator at {scale}x"
        );
        assert!(!response.state.hovered);
        assert!(!response.clicked);
    }
}
