//! Passive widget identity conformance tests for the [`Ui`] facade.

use kinetik_ui_core::{
    FrameOutput, ImageId, Rect, SemanticNode, SemanticRole, UiInput, UiMemory, WidgetId,
    default_dark_theme,
};
use kinetik_ui_widgets::Ui;

fn output_for(f: impl FnOnce(&mut Ui<'_>)) -> FrameOutput {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    f(&mut ui);
    ui.finish_output()
}

fn semantic_node<'a>(output: &'a FrameOutput, role: &SemanticRole) -> &'a SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.role == *role)
        .expect("semantic node")
}

#[test]
fn ui_keyed_label_semantic_id_is_stable_when_text_changes() {
    let first = output_for(|ui| {
        ui.label_keyed("status-label", Rect::new(0.0, 0.0, 80.0, 18.0), "Ready");
    });
    let second = output_for(|ui| {
        ui.label_keyed("status-label", Rect::new(0.0, 0.0, 80.0, 18.0), "Rendering");
    });

    let first = semantic_node(&first, &SemanticRole::Label);
    let second = semantic_node(&second, &SemanticRole::Label);
    assert_eq!(first.id, second.id);
    assert_eq!(first.label.as_deref(), Some("Ready"));
    assert_eq!(second.label.as_deref(), Some("Rendering"));
}

#[test]
fn ui_keyed_panel_semantic_id_is_stable_when_rect_changes() {
    let first = output_for(|ui| {
        ui.panel_keyed("inspector-panel", Rect::new(0.0, 0.0, 160.0, 80.0));
    });
    let second = output_for(|ui| {
        ui.panel_keyed("inspector-panel", Rect::new(24.0, 40.0, 220.0, 140.0));
    });

    let first = semantic_node(&first, &SemanticRole::Panel);
    let second = semantic_node(&second, &SemanticRole::Panel);
    assert_eq!(first.id, second.id);
    assert_ne!(first.bounds, second.bounds);
}

#[test]
fn ui_keyed_image_semantic_id_is_stable_when_rect_and_resource_change() {
    let first = output_for(|ui| {
        ui.image_keyed(
            "preview-image",
            Rect::new(0.0, 0.0, 64.0, 64.0),
            ImageId::from_raw(7),
        );
    });
    let second = output_for(|ui| {
        ui.image_keyed(
            "preview-image",
            Rect::new(8.0, 12.0, 96.0, 72.0),
            ImageId::from_raw(9),
        );
    });

    let first = semantic_node(&first, &SemanticRole::Image);
    let second = semantic_node(&second, &SemanticRole::Image);
    assert_eq!(first.id, second.id);
    assert_ne!(first.bounds, second.bounds);
    assert_eq!(first.label.as_deref(), Some("Image 7"));
    assert_eq!(second.label.as_deref(), Some("Image 9"));
}

#[test]
fn ui_legacy_passive_ids_still_track_presentation_data() {
    let label_a = output_for(|ui| {
        ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Ready");
    });
    let label_b = output_for(|ui| {
        ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Rendering");
    });
    assert_ne!(
        semantic_node(&label_a, &SemanticRole::Label).id,
        semantic_node(&label_b, &SemanticRole::Label).id
    );

    let panel_a = output_for(|ui| {
        ui.panel(Rect::new(0.0, 0.0, 160.0, 80.0));
    });
    let panel_b = output_for(|ui| {
        ui.panel(Rect::new(24.0, 40.0, 220.0, 140.0));
    });
    assert_ne!(
        semantic_node(&panel_a, &SemanticRole::Panel).id,
        semantic_node(&panel_b, &SemanticRole::Panel).id
    );

    let image_a = output_for(|ui| {
        ui.image(Rect::new(0.0, 0.0, 64.0, 64.0), ImageId::from_raw(7));
    });
    let image_b = output_for(|ui| {
        ui.image(Rect::new(8.0, 12.0, 96.0, 72.0), ImageId::from_raw(9));
    });
    assert_ne!(
        semantic_node(&image_a, &SemanticRole::Image).id,
        semantic_node(&image_b, &SemanticRole::Image).id
    );
}

#[test]
fn ui_keyed_passive_ids_match_explicit_widget_keys() {
    let output = output_for(|ui| {
        ui.label_keyed("status-label", Rect::new(0.0, 0.0, 80.0, 18.0), "Ready");
        ui.panel_keyed("inspector-panel", Rect::new(0.0, 24.0, 160.0, 80.0));
        ui.image_keyed(
            "preview-image",
            Rect::new(0.0, 120.0, 64.0, 64.0),
            ImageId::from_raw(7),
        );
    });

    assert_eq!(
        semantic_node(&output, &SemanticRole::Label).id,
        WidgetId::from_key("root").child("status-label")
    );
    assert_eq!(
        semantic_node(&output, &SemanticRole::Panel).id,
        WidgetId::from_key("root").child("inspector-panel")
    );
    assert_eq!(
        semantic_node(&output, &SemanticRole::Image).id,
        WidgetId::from_key("root").child("preview-image")
    );
}
