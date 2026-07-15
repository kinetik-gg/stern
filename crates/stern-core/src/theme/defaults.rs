use super::{
    ControlMetrics, DurationScale, ElevationScale, FontToken, OpacityScale, RadiusScale,
    SpacingScale, StrokeScale, Theme, ThemeColors, TypographyScale,
};

/// Returns the default dark editor theme.
#[must_use]
pub const fn default_dark_theme() -> Theme {
    let radii = RadiusScale::from_values(3.0, 6.0, 12.0, 9999.0);
    let strokes = StrokeScale::from_values(1.0, 1.0, 2.0, 1.0, 1.0);
    Theme {
        colors: ThemeColors::default_dark(),
        spacing: SpacingScale::new(2.0, 4.0, 8.0, 12.0, 16.0),
        radii,
        strokes,
        typography: TypographyScale {
            body: FontToken::new("Inter", 12.0, 17.0),
            label: FontToken::new("Inter", 12.0, 16.0),
            caption: FontToken::new("Inter", 11.0, 15.0),
            title: FontToken::new("Inter", 14.0, 19.0),
            monospace: FontToken::new("Geist Mono", 12.0, 17.0),
        },
        opacity: OpacityScale {
            disabled: 0.45,
            hover: 0.08,
            pressed: 0.14,
            selection: 0.35,
            overlay_scrim: 0.55,
        },
        elevation: ElevationScale::new(0.0, 1.0, 2.0, 3.0),
        duration: DurationScale {
            instant: 0.0,
            fast: 80.0,
            normal: 140.0,
            slow: 220.0,
        },
        controls: ControlMetrics {
            control_height: 28.0,
            compact_control_height: 22.0,
            icon_size: 16.0,
            check_size: 14.0,
            padding_x: 8.0,
            padding_y: 4.0,
        },
        radius: radii.sm,
        border_width: strokes.default,
        text_size: 12.0,
    }
}
