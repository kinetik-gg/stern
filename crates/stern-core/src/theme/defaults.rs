use super::{
    ControlMetrics, DurationScale, ElevationScale, FontToken, OpacityScale, RadiusScale,
    SpacingScale, Theme, ThemeColors, TypographyScale,
};
use crate::CornerRadius;

/// Returns the default dark editor theme.
#[must_use]
pub const fn default_dark_theme() -> Theme {
    Theme {
        colors: ThemeColors::default_dark(),
        spacing: SpacingScale::new(2.0, 4.0, 8.0, 12.0, 16.0),
        radii: RadiusScale::from_values(2.0, 3.0, 5.0, 8.0, 999.0),
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
        elevation: ElevationScale {
            flat: 0.0,
            raised: 1.0,
            overlay: 8.0,
        },
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
            border_width: 1.0,
            focus_width: 1.0,
            separator_width: 1.0,
        },
        radius: CornerRadius::all(3.0),
        border_width: 1.0,
        text_size: 12.0,
    }
}
