pub(super) use super::super::{
    ACTION_COMMAND_PALETTE, ACTION_COMPONENTS_RUN, ACTION_EDITOR_DOCK_JOIN,
    ACTION_SYSTEMS_DISPATCH, ACTION_VIEWPORT_GRID, ACTION_WORKSPACE_SAVE, ShowcaseApp,
    ShowcaseInput, ShowcasePage, frame_context, showcase_action_router, showcase_actions,
    static_render_resources,
};
pub(super) use crate::editor::phosphor_icons;
pub(super) use kinetik_ui::{
    core::{
        ActionContext, ActionId, ActionInvocation, ActionSource, ImageId, Key, KeyEvent, KeyState,
        KeyboardInput, Modifiers, PhysicalSize, PlatformRequest, Point, Primitive, Rect,
        RepaintRequest, ScaleFactor, SemanticActionKind, SemanticRole, SemanticValue, Size,
        TextureId, UiInput, ViewportInfo, WidgetId,
    },
    render::{RenderFrameInput, RenderImageSampling},
    render_vello::VelloRenderer,
};

pub(super) fn click(app: &mut ShowcaseApp, point: Point) {
    app.update(&ShowcaseInput {
        mouse: Some(point),
        mouse_down: true,
        ..ShowcaseInput::default()
    });
    app.update(&ShowcaseInput {
        mouse: Some(point),
        mouse_down: false,
        ..ShowcaseInput::default()
    });
}

pub(super) fn has_text(app: &ShowcaseApp, value: &str) -> bool {
    app.primitives()
        .iter()
        .any(|primitive| matches!(primitive, Primitive::Text(text) if text.text == value))
}

pub(super) fn count_primitives(app: &ShowcaseApp, predicate: impl Fn(&Primitive) -> bool) -> usize {
    app.output()
        .primitives
        .iter()
        .filter(|primitive| predicate(primitive))
        .count()
}

pub(super) fn count_semantic_role(app: &ShowcaseApp, role: &SemanticRole) -> usize {
    app.output()
        .semantics
        .nodes()
        .iter()
        .filter(|node| &node.role == role)
        .count()
}

pub(super) fn semantic_node(app: &ShowcaseApp, role: &SemanticRole, label: &str) -> bool {
    app.output()
        .semantics
        .nodes()
        .iter()
        .any(|node| &node.role == role && node.label.as_deref() == Some(label))
}

pub(super) fn semantic_role_has_action(
    app: &ShowcaseApp,
    role: &SemanticRole,
    action: &SemanticActionKind,
) -> bool {
    app.output()
        .semantics
        .nodes()
        .iter()
        .any(|node| &node.role == role && node.actions.iter().any(|item| &item.kind == action))
}

pub(super) fn text_labels(app: &ShowcaseApp) -> Vec<&str> {
    app.output()
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect()
}

pub(super) fn contains_text_in_order(app: &ShowcaseApp, expected: &[&str]) -> bool {
    let mut cursor = 0;
    for label in text_labels(app) {
        if expected
            .get(cursor)
            .is_some_and(|expected| *expected == label)
        {
            cursor += 1;
            if cursor == expected.len() {
                return true;
            }
        }
    }
    false
}

pub(super) fn viewport_texture_rect(app: &ShowcaseApp) -> Rect {
    app.primitives()
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Texture(texture) if texture.texture == TextureId::from_raw(99) => {
                Some(texture.rect)
            }
            _ => None,
        })
        .expect("viewport texture")
}

pub(super) fn test_viewport(size: Size) -> ViewportInfo {
    test_viewport_scaled(size, 1.0)
}

pub(super) fn test_viewport_scaled(size: Size, scale_factor: f64) -> ViewportInfo {
    ViewportInfo::new(
        size,
        PhysicalSize::new(
            (f64::from(size.width) * scale_factor).round().max(1.0) as u32,
            (f64::from(size.height) * scale_factor).round().max(1.0) as u32,
        ),
        ScaleFactor::new(scale_factor),
    )
}
