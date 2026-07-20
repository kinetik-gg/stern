//! Consumer-view compatibility contracts for prepared viewport values.

use stern::widgets::viewport::{
    ViewportPresentation, ViewportSurface, ViewportToolScene, ViewportWidget, ViewportWidgetOutput,
};

fn assert_send_sync<T: Send + Sync>() {}

const fn frozen_widget_surface(widget: &ViewportWidget) -> ViewportSurface {
    widget.surface()
}

const fn frozen_scene_surface(scene: &ViewportToolScene) -> ViewportSurface {
    scene.surface()
}

fn reconstruct_widget_output(output: ViewportWidgetOutput) -> ViewportWidgetOutput {
    let ViewportWidgetOutput {
        response,
        surface,
        next_pan_zoom,
        content_pointer,
        pan_changed,
        zoom_changed,
        fit_changed,
        action_requests,
    } = output;
    ViewportWidgetOutput {
        response,
        surface,
        next_pan_zoom,
        content_pointer,
        pan_changed,
        zoom_changed,
        fit_changed,
        action_requests,
    }
}

#[test]
fn prepared_viewport_values_preserve_thread_const_and_output_shape_contracts() {
    assert_send_sync::<ViewportWidget>();
    assert_send_sync::<ViewportToolScene>();
    assert_send_sync::<ViewportPresentation>();

    let _: fn(&ViewportWidget) -> ViewportSurface = frozen_widget_surface;
    let _: fn(&ViewportToolScene) -> ViewportSurface = frozen_scene_surface;
    let _: fn(ViewportWidgetOutput) -> ViewportWidgetOutput = reconstruct_widget_output;
}
