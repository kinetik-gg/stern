use super::common::assert_approx64;
use crate::{
    RenderDiagnostic, RenderFrameInput, RenderResources, RendererBackend, VelloRenderer,
    VelloRendererError, viewport_device_scale, viewport_size_device_scale,
};
use kinetik_ui_core::{ImageId, ImagePrimitive, Primitive, Rect, ScaleFactor, Size, ViewportInfo};

#[test]
fn frame_submission_reports_primitive_count_and_diagnostics() {
    let mut renderer = VelloRenderer::new();
    let primitives = vec![Primitive::Image(ImagePrimitive {
        image: ImageId::from_raw(9),
        rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        tint: None,
    })];
    let resources = RenderResources::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });

    assert_eq!(output.primitive_count, 1);
    assert_eq!(
        output.diagnostics,
        vec![RenderDiagnostic::MissingImage(ImageId::from_raw(9))]
    );
}

#[test]
fn renderer_backend_trait_submits_vello_frames() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();

    let output = RendererBackend::render_frame(
        &mut renderer,
        RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(100, 100),
                ScaleFactor::ONE,
            ),
            primitives: &[],
            resources: &resources,
        },
    )
    .expect("Vello CPU scene encoding should not return fatal submission errors");

    assert_eq!(output.primitive_count, 0);
    assert!(output.diagnostics.is_empty());
    assert!(renderer.scene().encoding().is_empty());
}

#[test]
fn renderer_backend_uses_concrete_vello_error_type() {
    fn assert_error_type<T: RendererBackend<Error = VelloRendererError>>(_: &T) {}

    let renderer = VelloRenderer::new();

    assert_error_type(&renderer);
}

#[test]
fn viewport_device_scale_uses_frame_scale_factor() {
    let viewport = ViewportInfo::new(
        Size::new(800.0, 600.0),
        kinetik_ui_core::PhysicalSize::new(1200, 900),
        ScaleFactor::new(1.5),
    );

    assert!((viewport_device_scale(viewport) - 1.5).abs() < f64::EPSILON);
}

#[test]
fn viewport_device_scale_prefers_uniform_framebuffer_scale() {
    let viewport = ViewportInfo::new(
        Size::new(800.0, 600.0),
        kinetik_ui_core::PhysicalSize::new(1000, 750),
        ScaleFactor::new(1.0),
    );

    assert_approx64(
        viewport_size_device_scale(viewport).expect("size scale"),
        1.25,
    );
    assert_approx64(viewport_device_scale(viewport), 1.25);
}

#[test]
fn viewport_device_scale_falls_back_when_framebuffer_axes_disagree() {
    let viewport = ViewportInfo::new(
        Size::new(800.0, 600.0),
        kinetik_ui_core::PhysicalSize::new(1000, 720),
        ScaleFactor::new(1.5),
    );

    assert_eq!(viewport_size_device_scale(viewport), None);
    assert_approx64(viewport_device_scale(viewport), 1.5);
}
