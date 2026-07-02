use super::{
    ControlMetrics, DurationScale, ElevationScale, FontToken, OpacityScale, RadiusScale,
    SpacingScale, Theme, ThemeColors, TypographyScale,
};
use crate::{Color, CornerRadius};

/// Returns the default dark editor theme.
#[must_use]
pub const fn default_dark_theme() -> Theme {
    Theme {
        colors: ThemeColors {
            surface: Color::rgb(0.055, 0.055, 0.055),
            surface_raised: Color::rgb(0.085, 0.085, 0.085),
            surface_hover: Color::rgb(0.13, 0.13, 0.13),
            surface_active: Color::rgb(0.16, 0.16, 0.16),
            surface_sunken: Color::rgb(0.035, 0.035, 0.035),
            text: Color::rgb(0.86, 0.86, 0.86),
            text_muted: Color::rgb(0.52, 0.52, 0.52),
            text_disabled: Color::rgb(0.30, 0.30, 0.30),
            accent: Color::rgb(0.13, 0.40, 0.96),
            danger: Color::rgb(0.86, 0.22, 0.22),
            warning: Color::rgb(0.90, 0.62, 0.18),
            success: Color::rgb(0.26, 0.70, 0.38),
            border: Color::rgb(0.21, 0.21, 0.21),
            border_subtle: Color::rgb(0.14, 0.14, 0.14),
            focus_ring: Color::rgb(0.25, 0.55, 1.0),
            selection: Color::rgb(0.13, 0.40, 0.96),
            disabled: Color::rgb(0.075, 0.075, 0.075),
            overlay: Color::rgb(0.105, 0.105, 0.105),
            viewport_background: Color::rgb(0.02, 0.02, 0.02),
        },
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
