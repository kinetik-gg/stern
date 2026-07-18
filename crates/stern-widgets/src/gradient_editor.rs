//! Retained, renderer-honest gradient editor composition.
#![allow(missing_docs)]

use stern_core::{Color, GradientStop, LinearGradient, Point, Rect, Response, WidgetId};

macro_rules! config_accessors {
    ($(($name:ident, $field:ident, $ty:ty)),+ $(,)?) => {
        $(
            #[must_use]
            pub const fn $name(&self) -> $ty {
                self.config.$field
            }
        )+
    };
}

macro_rules! config_builders {
    ($(($name:ident, $field:ident, $ty:ty)),+ $(,)?) => {
        $(
            #[must_use]
            pub const fn $name(mut self, value: $ty) -> Self {
                self.$field = value;
                self
            }
        )+
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientInterpolationSpace {
    Srgb,
    LinearSrgb,
    DisplayP3,
}

impl GradientInterpolationSpace {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Srgb => "sRGB",
            Self::LinearSrgb => "Linear sRGB",
            Self::DisplayP3 => "Display-P3",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GradientEditorStopId(u64);

impl GradientEditorStopId {
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientEditorStop {
    pub id: GradientEditorStopId,
    pub position: f32,
    pub color: Color,
    pub removable: bool,
}

impl GradientEditorStop {
    #[must_use]
    pub const fn new(id: GradientEditorStopId, position: f32, color: Color) -> Self {
        Self {
            id,
            position,
            color,
            removable: false,
        }
    }

    #[must_use]
    pub const fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientEditorConfig<'a> {
    pub(crate) id: WidgetId,
    pub(crate) bounds: Rect,
    pub(crate) space: GradientInterpolationSpace,
    pub(crate) stops: &'a [GradientEditorStop],
    pub(crate) selected: Option<GradientEditorStopId>,
    pub(crate) disabled: bool,
    pub(crate) read_only: bool,
    pub(crate) keyboard_step: f32,
}

impl<'a> GradientEditorConfig<'a> {
    #[must_use]
    pub const fn new(
        id: WidgetId,
        bounds: Rect,
        space: GradientInterpolationSpace,
        stops: &'a [GradientEditorStop],
    ) -> Self {
        Self {
            id,
            bounds,
            space,
            stops,
            selected: None,
            disabled: false,
            read_only: false,
            keyboard_step: 0.01,
        }
    }

    #[must_use]
    pub const fn selected_stop(mut self, selected: GradientEditorStopId) -> Self {
        self.selected = Some(selected);
        self
    }

    config_builders!(
        (disabled, disabled, bool),
        (read_only, read_only, bool),
        (keyboard_step, keyboard_step, f32),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientEditorPrepareError {
    InvalidStopCount,
    DuplicateStopId,
    UnknownSelectedStop,
    InvalidStopPosition,
    InvalidStopColor,
    UnsupportedInterpolationSpace,
    TranslucentPreview,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientEditorWidget<'a> {
    pub(crate) config: GradientEditorConfig<'a>,
    pub(crate) gradient: LinearGradient,
}

impl<'a> GradientEditorWidget<'a> {
    pub(crate) fn prepare(
        config: GradientEditorConfig<'a>,
    ) -> Result<Self, GradientEditorPrepareError> {
        validate(config)?;
        let ramp = ramp_rect(config.bounds);
        let stops = config
            .stops
            .iter()
            .map(|stop| GradientStop::new(stop.position, stop.color))
            .collect::<Vec<_>>();
        let gradient = LinearGradient::new(
            Point::new(ramp.x, ramp.y),
            Point::new(ramp.max_x(), ramp.y),
            &stops,
        )
        .map_err(|_| GradientEditorPrepareError::InvalidStopCount)?;
        Ok(Self { config, gradient })
    }

    config_accessors!(
        (widget_id, id, WidgetId),
        (bounds, bounds, Rect),
        (space, space, GradientInterpolationSpace),
        (selected_stop, selected, Option<GradientEditorStopId>),
        (disabled, disabled, bool),
        (read_only, read_only, bool),
    );

    #[must_use]
    pub fn stops(&self) -> &[GradientEditorStop] {
        self.config.stops
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GradientEditorIntent {
    SelectStop(GradientEditorStopId),
    MoveStop {
        id: GradientEditorStopId,
        position: f32,
    },
    RemoveStop(GradientEditorStopId),
    Reverse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientEditorOutput {
    pub response: Response,
    pub intents: Vec<GradientEditorIntent>,
}

pub(crate) fn ramp_rect(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 8.0, bounds.y + 28.0, bounds.width - 16.0, 28.0)
}

pub(crate) fn marker_rect(bounds: Rect, stop: GradientEditorStop) -> Rect {
    let ramp = ramp_rect(bounds);
    let x = ramp.x + ramp.width * stop.position - 6.0;
    Rect::new(x, ramp.y + 6.0, 12.0, 16.0)
}

pub(crate) fn stop_widget_id(root: WidgetId, id: GradientEditorStopId) -> WidgetId {
    root.child(("gradient-stop", id.raw()))
}

fn validate(config: GradientEditorConfig<'_>) -> Result<(), GradientEditorPrepareError> {
    if !(2..=stern_core::MAX_GRADIENT_STOPS).contains(&config.stops.len()) {
        return Err(GradientEditorPrepareError::InvalidStopCount);
    }
    if config.space != GradientInterpolationSpace::Srgb {
        return Err(GradientEditorPrepareError::UnsupportedInterpolationSpace);
    }
    for (index, stop) in config.stops.iter().enumerate() {
        if config.stops[..index].iter().any(|seen| seen.id == stop.id) {
            return Err(GradientEditorPrepareError::DuplicateStopId);
        }
    }
    for stop in config.stops {
        if !(0.0..=1.0).contains(&stop.position) {
            return Err(GradientEditorPrepareError::InvalidStopPosition);
        }
        let invalid_color = [stop.color.r, stop.color.g, stop.color.b, stop.color.a]
            .into_iter()
            .any(|channel| !(0.0..=1.0).contains(&channel));
        if invalid_color {
            return Err(GradientEditorPrepareError::InvalidStopColor);
        }
        if stop.color.a < 1.0 {
            return Err(GradientEditorPrepareError::TranslucentPreview);
        }
    }
    if let Some(selected) = config.selected
        && !config.stops.iter().any(|stop| stop.id == selected)
    {
        return Err(GradientEditorPrepareError::UnknownSelectedStop);
    }
    Ok(())
}
