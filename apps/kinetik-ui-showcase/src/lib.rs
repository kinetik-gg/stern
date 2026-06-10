//! Deterministic showcase scenarios for Kinetik UI.

use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, ImageId, Point, Primitive, Rect, RectPrimitive, Stroke,
    TextureId, Theme, UiInput, UiMemory, WidgetId, default_dark_theme,
};
use kinetik_ui_text::TextEditState;
use kinetik_ui_widgets::{
    Crosshair, Guide, PanZoom, ViewportComposition, ViewportSurface, button, checkbox, image,
    label, numeric_input, panel, search_field, slider, text_field, toggle,
};

/// Showcase scenario output.
#[derive(Debug, Clone, PartialEq)]
pub struct ShowcaseScenario {
    /// Scenario name.
    pub name: String,
    /// Generated primitive stream.
    pub primitives: Vec<Primitive>,
}

impl ShowcaseScenario {
    /// Creates a scenario.
    #[must_use]
    pub fn new(name: impl Into<String>, primitives: Vec<Primitive>) -> Self {
        Self {
            name: name.into(),
            primitives,
        }
    }
}

/// Builds an editor shell scenario inspired by the reference layout.
#[must_use]
pub fn editor_shell() -> ShowcaseScenario {
    let theme = default_dark_theme();
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect: Rect::new(0.0, 0.0, 1440.0, 900.0),
        fill: Some(Brush::Solid(Color::rgb(0.06, 0.06, 0.06))),
        stroke: None,
        radius: CornerRadius::all(0.0),
    })];

    primitives.extend(panel(Rect::new(0.0, 40.0, 300.0, 820.0), &theme).primitives);
    primitives.extend(panel(Rect::new(300.0, 40.0, 1140.0, 720.0), &theme).primitives);
    primitives.extend(panel(Rect::new(300.0, 760.0, 1140.0, 100.0), &theme).primitives);
    primitives
        .extend(label(Rect::new(12.0, 60.0, 180.0, 20.0), "Media Library", &theme).primitives);
    primitives.extend(
        label(
            Rect::new(320.0, 62.0, 180.0, 20.0),
            "My media [1...24].exr",
            &theme,
        )
        .primitives,
    );

    let viewport = ViewportComposition {
        surface: ViewportSurface {
            texture: TextureId::from_raw(1),
            source_size: kinetik_ui_core::Size::new(4096.0, 1716.0),
            bounds: Rect::new(330.0, 120.0, 1060.0, 560.0),
            pan_zoom: PanZoom::default(),
        },
        guides: vec![Guide::Horizontal(400.0), Guide::Vertical(1080.0)],
        crosshair: Some(Crosshair {
            visible: true,
            position: Point::new(1080.0, 400.0),
            label: None,
            color: Color::rgba(1.0, 1.0, 1.0, 0.5),
        }),
        clip: ClipId::from_raw(1),
    };
    primitives.extend(viewport.primitives());

    primitives.push(Primitive::Line(kinetik_ui_core::LinePrimitive {
        from: Point::new(300.0, 760.0),
        to: Point::new(1440.0, 760.0),
        stroke: Stroke::new(1.0, Brush::Solid(theme.colors.border)),
    }));

    ShowcaseScenario::new("editor-shell", primitives)
}

/// Builds a component gallery scenario.
#[must_use]
pub fn component_gallery() -> ShowcaseScenario {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let input = UiInput::default();
    let mut primitives = Vec::new();

    primitives.extend(label(Rect::new(16.0, 16.0, 160.0, 20.0), "Components", &theme).primitives);
    primitives.extend(component_gallery_controls(&theme, &input, &mut memory));
    primitives.extend(image(Rect::new(220.0, 48.0, 32.0, 32.0), ImageId::from_raw(7)).primitives);

    ShowcaseScenario::new("component-gallery", primitives)
}

fn component_gallery_controls(
    theme: &Theme,
    input: &UiInput,
    memory: &mut UiMemory,
) -> Vec<Primitive> {
    let mut text = TextEditState::new("Project");
    let mut number = TextEditState::new("0.62");
    let mut search = TextEditState::new("media");
    let mut slider_value = 0.62;
    let mut primitives = Vec::new();

    primitives.extend(
        button(
            WidgetId::from_key("run"),
            Rect::new(16.0, 48.0, 96.0, 28.0),
            "Analyze",
            input,
            memory,
            theme,
            false,
        )
        .primitives,
    );
    primitives.extend(
        checkbox(
            WidgetId::from_key("check"),
            Rect::new(16.0, 90.0, 20.0, 20.0),
            true,
            input,
            memory,
            theme,
            false,
        )
        .primitives,
    );
    primitives.extend(
        toggle(
            WidgetId::from_key("toggle"),
            Rect::new(16.0, 126.0, 44.0, 20.0),
            true,
            input,
            memory,
            theme,
            false,
        )
        .primitives,
    );
    primitives.extend(
        slider(
            WidgetId::from_key("slider"),
            Rect::new(16.0, 164.0, 160.0, 14.0),
            &mut slider_value,
            0.0..=1.0,
            input,
            memory,
            theme,
            false,
        )
        .primitives,
    );
    primitives.extend(
        text_field(
            WidgetId::from_key("text"),
            Rect::new(16.0, 196.0, 180.0, 26.0),
            &mut text,
            input,
            memory,
            theme,
            false,
        )
        .widget
        .primitives,
    );
    primitives.extend(
        numeric_input(
            WidgetId::from_key("number"),
            Rect::new(16.0, 232.0, 180.0, 26.0),
            &mut number,
            input,
            memory,
            theme,
            false,
        )
        .field
        .widget
        .primitives,
    );
    primitives.extend(
        search_field(
            WidgetId::from_key("search"),
            Rect::new(16.0, 268.0, 180.0, 26.0),
            &mut search,
            input,
            memory,
            theme,
            false,
        )
        .field
        .widget
        .primitives,
    );

    primitives
}

/// Builds a viewport scenario.
#[must_use]
pub fn viewport_scenario() -> ShowcaseScenario {
    let viewport = ViewportComposition {
        surface: ViewportSurface {
            texture: TextureId::from_raw(9),
            source_size: kinetik_ui_core::Size::new(1920.0, 1080.0),
            bounds: Rect::new(0.0, 0.0, 640.0, 360.0),
            pan_zoom: PanZoom::default(),
        },
        guides: vec![
            Guide::Horizontal(120.0),
            Guide::Horizontal(240.0),
            Guide::Vertical(320.0),
        ],
        crosshair: Some(Crosshair {
            visible: true,
            position: Point::new(320.0, 180.0),
            label: Some("320,180".to_owned()),
            color: Color::WHITE,
        }),
        clip: ClipId::from_raw(9),
    };

    ShowcaseScenario::new("viewport", viewport.primitives())
}

/// Returns every showcase scenario.
#[must_use]
pub fn all_scenarios() -> Vec<ShowcaseScenario> {
    vec![editor_shell(), component_gallery(), viewport_scenario()]
}

#[cfg(test)]
mod tests {
    use super::{all_scenarios, component_gallery, editor_shell, viewport_scenario};
    use kinetik_ui_core::Primitive;

    #[test]
    fn scenarios_have_primitives() {
        for scenario in all_scenarios() {
            assert!(!scenario.primitives.is_empty(), "{}", scenario.name);
        }
    }

    #[test]
    fn editor_shell_contains_texture_viewport() {
        assert!(
            editor_shell()
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, Primitive::Texture(_)))
        );
    }

    #[test]
    fn component_gallery_contains_text_and_controls() {
        let scenario = component_gallery();
        assert!(
            scenario
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, Primitive::Text(_)))
        );
        assert!(
            scenario
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, Primitive::Rect(_)))
        );
    }

    #[test]
    fn viewport_scenario_is_clipped() {
        let scenario = viewport_scenario();
        assert!(matches!(
            scenario.primitives.first(),
            Some(Primitive::ClipBegin { .. })
        ));
        assert!(matches!(
            scenario.primitives.last(),
            Some(Primitive::ClipEnd { .. })
        ));
    }
}
