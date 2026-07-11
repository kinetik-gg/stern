use std::time::Duration;

use crate::debug::{DiagnosticCategory, DiagnosticLocation, DiagnosticSeverity, FrameDiagnostic};
use crate::input::{InputStreamConflict, UiInput};
use crate::render::{ClipId, LayerId};
use crate::{PhysicalSize, Rect, ScaleFactor, SemanticTreeError, Size, WidgetId};

/// Information about the current rendering viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportInfo {
    /// Size used by UI layout.
    pub logical_size: Size,
    /// Size of the physical render target.
    pub physical_size: PhysicalSize,
    /// Scale factor between logical and physical units.
    pub scale_factor: ScaleFactor,
}

impl ViewportInfo {
    /// Creates viewport information.
    #[must_use]
    pub const fn new(
        logical_size: Size,
        physical_size: PhysicalSize,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            logical_size,
            physical_size,
            scale_factor,
        }
    }
}

/// Time information for one UI frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TimeInfo {
    /// Monotonic timestamp relative to the application-defined start.
    pub now: Duration,
    /// Time since the previous frame.
    pub delta: Duration,
    /// Sequential frame number.
    pub frame_index: u64,
}

impl TimeInfo {
    /// Creates frame time information.
    #[must_use]
    pub const fn new(now: Duration, delta: Duration, frame_index: u64) -> Self {
        Self {
            now,
            delta,
            frame_index,
        }
    }
}

/// Context provided to the UI runtime at the beginning of a frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameContext {
    /// Viewport and DPI information.
    pub viewport: ViewportInfo,
    /// Input snapshot for this frame.
    pub input: UiInput,
    /// Time snapshot for this frame.
    pub time: TimeInfo,
}

impl FrameContext {
    /// Creates a frame context.
    #[must_use]
    pub const fn new(viewport: ViewportInfo, input: UiInput, time: TimeInfo) -> Self {
        Self {
            viewport,
            input,
            time,
        }
    }
}

/// Request for when the platform adapter should schedule another redraw.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RepaintRequest {
    /// No repaint is currently needed.
    #[default]
    None,
    /// Repaint as soon as the platform can present another frame.
    NextFrame,
    /// Repaint after the provided delay.
    After(Duration),
    /// Continue repainting while an external active condition remains true.
    Continuous,
}

impl RepaintRequest {
    /// Combines two repaint requests, preserving the more urgent request.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Continuous, _) | (_, Self::Continuous) => Self::Continuous,
            (Self::NextFrame, _) | (_, Self::NextFrame) => Self::NextFrame,
            (Self::After(a), Self::After(b)) => Self::After(a.min(b)),
            (Self::After(delay), Self::None) | (Self::None, Self::After(delay)) => {
                Self::After(delay)
            }
            (Self::None, Self::None) => Self::None,
        }
    }
}

/// Cursor shape requested by toolkit code.
///
/// Platform adapters translate these neutral shapes to the host cursor API.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CursorShape {
    /// Platform default cursor.
    #[default]
    Default,
    /// Text insertion cursor.
    Text,
    /// Clickable item cursor.
    PointingHand,
    /// Crosshair cursor.
    Crosshair,
    /// Open hand drag cursor.
    Grab,
    /// Closed hand drag cursor.
    Grabbing,
    /// Horizontal resize cursor.
    ResizeHorizontal,
    /// Vertical resize cursor.
    ResizeVertical,
    /// Diagonal resize from top-left to bottom-right.
    ResizeTopLeftBottomRight,
    /// Diagonal resize from top-right to bottom-left.
    ResizeTopRightBottomLeft,
    /// Operation is unavailable.
    NotAllowed,
}

/// Platform-neutral request emitted by toolkit code during a frame.
///
/// The core crate records intent only. Windowing, clipboard, IME, browser, and
/// shell integration stay in platform/application adapters.
#[derive(Debug, Clone, PartialEq)]
pub enum PlatformRequest {
    /// Set the pointer cursor for the current frame.
    SetCursor(CursorShape),
    /// Copy text to the platform clipboard.
    CopyToClipboard(String),
    /// Ask the platform adapter to provide clipboard text as future input.
    RequestClipboardText {
        /// Text-input widget that should receive the clipboard text.
        target: WidgetId,
    },
    /// Start platform text input or IME at an optional logical text-editing rect.
    StartTextInput {
        /// Logical rectangle for caret/composition placement.
        rect: Option<Rect>,
    },
    /// Stop platform text input or IME.
    StopTextInput,
    /// Set the host window title.
    SetWindowTitle(String),
    /// Ask the application/platform shell to open a URL.
    OpenUrl(String),
}

/// Runtime warning detected while finalizing a UI frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameWarning {
    /// Canonical input events conflict with their compatibility projections.
    InputStreamConflict {
        /// First mismatch in the deterministic projection validation order.
        conflict: InputStreamConflict,
    },
    /// The same widget ID was registered more than once in one frame.
    DuplicateWidgetId {
        /// Duplicated widget identity.
        id: WidgetId,
    },
    /// A clip end command did not match the current open clip.
    UnmatchedClipEnd {
        /// Clip ID carried by the unmatched end command.
        id: ClipId,
    },
    /// A clip begin command remained open at the end of the frame.
    UnclosedClip {
        /// Clip ID left open.
        id: ClipId,
    },
    /// A layer end command did not match the current open layer.
    UnmatchedLayerEnd {
        /// Layer ID carried by the unmatched end command.
        id: LayerId,
    },
    /// A layer begin command remained open at the end of the frame.
    UnclosedLayer {
        /// Layer ID left open.
        id: LayerId,
    },
    /// A transform end command appeared without a matching begin.
    UnmatchedTransformEnd,
    /// Transform begin commands remained open at the end of the frame.
    UnclosedTransforms {
        /// Number of unclosed transform scopes.
        count: usize,
    },
    /// Accessibility semantic tree failed structural validation.
    InvalidSemanticTree {
        /// Structural validation error.
        error: SemanticTreeError,
    },
}

impl FrameWarning {
    /// Returns stable structured diagnostic metadata for this warning.
    #[must_use]
    pub const fn diagnostic(&self) -> FrameDiagnostic {
        match *self {
            Self::InputStreamConflict { .. } => FrameDiagnostic {
                code: "input.stream_projection_conflict",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::Input,
                location: DiagnosticLocation::InputStream,
            },
            Self::DuplicateWidgetId { id } => FrameDiagnostic {
                code: "identity.duplicate_widget_id",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::Identity,
                location: DiagnosticLocation::Widget(id),
            },
            Self::UnmatchedClipEnd { id } => FrameDiagnostic {
                code: "primitive_stack.unmatched_clip_end",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::Clip(id),
            },
            Self::UnclosedClip { id } => FrameDiagnostic {
                code: "primitive_stack.unclosed_clip",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::Clip(id),
            },
            Self::UnmatchedLayerEnd { id } => FrameDiagnostic {
                code: "primitive_stack.unmatched_layer_end",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::Layer(id),
            },
            Self::UnclosedLayer { id } => FrameDiagnostic {
                code: "primitive_stack.unclosed_layer",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::Layer(id),
            },
            Self::UnmatchedTransformEnd => FrameDiagnostic {
                code: "primitive_stack.unmatched_transform_end",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::TransformStack,
            },
            Self::UnclosedTransforms { .. } => FrameDiagnostic {
                code: "primitive_stack.unclosed_transforms",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::TransformStack,
            },
            Self::InvalidSemanticTree { .. } => FrameDiagnostic {
                code: "semantics.invalid_tree",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::SemanticTree,
                location: DiagnosticLocation::SemanticTree,
            },
        }
    }
}
