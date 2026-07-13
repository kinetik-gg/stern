//! Builds one deterministic UI frame through the application-facing facade.

use std::time::Duration;

use kinetik_ui::{
    core::{Insets, LayoutItem, Measurement, SizeRule},
    prelude::*,
};

fn main() {
    let theme = default_dark_theme();
    let viewport = ViewportInfo::new(
        Size::new(960.0, 540.0),
        PhysicalSize::new(1920, 1080),
        ScaleFactor::new(2.0),
    );
    let context = FrameContext::new(
        viewport,
        UiInput::default(),
        TimeInfo::new(Duration::ZERO, Duration::from_millis(16), 0),
    );

    let mut state = UiState::new();
    let mut query = TextEditState::new("media");
    let mut amount = 0.45;
    let run_action = ActionDescriptor::new("run", "Run");

    let mut ui = state.begin_frame(context, &theme);
    let panel = Rect::new(24.0, 24.0, 360.0, 184.0);
    ui.panel(panel);

    let fixed = |width, height| {
        LayoutItem::new(
            SizeRule::Fixed(width),
            SizeRule::Fixed(height),
            Measurement::default(),
        )
    };
    let layout_items = [
        fixed(240.0, 24.0),
        fixed(0.0, 8.0),
        fixed(96.0, 30.0),
        fixed(0.0, 18.0),
        fixed(220.0, 16.0),
        fixed(0.0, 16.0),
        fixed(240.0, 28.0),
    ];
    let mut run = None;
    let mut scrub = None;
    let mut search = None;
    ui.padding("content", panel, Insets::all(16.0), |ui, content| {
        ui.column(
            "controls",
            content,
            &layout_items,
            0.0,
            |ui, index, rect| match index {
                0 => ui.label(rect, "Kinetik UI"),
                2 => {
                    run = Some(
                        ui.action_button("run", rect, &run_action, ActionContext::Global)
                            .expect("run action is visible"),
                    );
                }
                4 => {
                    scrub = Some(ui.slider("amount", rect, &mut amount, 0.0..=1.0, false));
                }
                6 => {
                    search = Some(ui.search_field("search", rect, &mut query, false));
                }
                _ => {}
            },
        );
    });
    let run = run.expect("run action was allocated");
    let scrub = scrub.expect("amount slider was allocated");
    let search = search.expect("search field was allocated");
    let output = ui.finish_output();

    assert!(!run.clicked);
    assert_eq!(scrub.rect, Rect::new(40.0, 120.0, 220.0, 16.0));
    assert_eq!(search.query, "media");
    assert!(!output.primitives.is_empty());
    assert!(output.semantics.validate().is_ok());
    assert!(output.warnings.is_empty());
    assert!(!state.text_layouts().is_empty());
}
