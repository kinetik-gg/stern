use crate::{Color, CornerRadius, Rect, ShadowPrimitive, Vec2};

/// Exact semantic color token key.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticColor {
    /// `surface.application`.
    SurfaceApplication,
    /// `surface.workspace`.
    SurfaceWorkspace,
    /// `surface.panel`.
    SurfacePanel,
    /// `surface.panel_raised`.
    SurfacePanelRaised,
    /// `surface.raised`.
    SurfaceRaised,
    /// `surface.control`.
    SurfaceControl,
    /// `surface.control_hover`.
    SurfaceControlHover,
    /// `surface.control_pressed`.
    SurfaceControlPressed,
    /// `surface.control_disabled`.
    SurfaceControlDisabled,
    /// `surface.overlay`.
    SurfaceOverlay,
    /// `surface.hover`.
    SurfaceHover,
    /// `surface.sunken`.
    SurfaceSunken,
    /// `content.primary`.
    ContentPrimary,
    /// `content.secondary`.
    ContentSecondary,
    /// `content.muted`.
    ContentMuted,
    /// `content.disabled`.
    ContentDisabled,
    /// `content.on_accent`.
    ContentOnAccent,
    /// `content.link`.
    ContentLink,
    /// `border.subtle`.
    BorderSubtle,
    /// `border.default`.
    BorderDefault,
    /// `border.strong`.
    BorderStrong,
    /// `border.hover`.
    BorderHover,
    /// `border.focused`.
    BorderFocused,
    /// `border.disabled`.
    BorderDisabled,
    /// `border.invalid`.
    BorderInvalid,
    /// `selection.background`.
    SelectionBackground,
    /// `selection.foreground`.
    SelectionForeground,
    /// `focus.indicator`.
    FocusIndicator,
    /// `focus.separator`.
    FocusSeparator,
    /// `focus.ring`.
    FocusRing,
    /// `overlay.scrim`.
    OverlayScrim,
    /// `accent.subtle`.
    AccentSubtle,
    /// `accent.default`.
    AccentDefault,
    /// `accent.hover`.
    AccentHover,
    /// `accent.pressed`.
    AccentPressed,
    /// `accent.focus`.
    AccentFocus,
    /// `accent.foreground`.
    AccentForeground,
    /// `status.info.foreground`.
    StatusInfoForeground,
    /// `status.info.surface`.
    StatusInfoSurface,
    /// `status.info.border`.
    StatusInfoBorder,
    /// `status.info.strong`.
    StatusInfoStrong,
    /// `status.success.foreground`.
    StatusSuccessForeground,
    /// `status.success.surface`.
    StatusSuccessSurface,
    /// `status.success.border`.
    StatusSuccessBorder,
    /// `status.success.strong`.
    StatusSuccessStrong,
    /// `status.warning.foreground`.
    StatusWarningForeground,
    /// `status.warning.surface`.
    StatusWarningSurface,
    /// `status.warning.border`.
    StatusWarningBorder,
    /// `status.warning.strong`.
    StatusWarningStrong,
    /// `status.danger.foreground`.
    StatusDangerForeground,
    /// `status.danger.surface`.
    StatusDangerSurface,
    /// `status.danger.border`.
    StatusDangerBorder,
    /// `status.danger.strong`.
    StatusDangerStrong,
}

impl SemanticColor {
    /// Every stored semantic color token in stable grouped-field order.
    pub const ALL: &'static [Self] = &[
        Self::SurfaceApplication,
        Self::SurfaceWorkspace,
        Self::SurfacePanel,
        Self::SurfacePanelRaised,
        Self::SurfaceRaised,
        Self::SurfaceControl,
        Self::SurfaceControlHover,
        Self::SurfaceControlPressed,
        Self::SurfaceControlDisabled,
        Self::SurfaceOverlay,
        Self::SurfaceHover,
        Self::SurfaceSunken,
        Self::ContentPrimary,
        Self::ContentSecondary,
        Self::ContentMuted,
        Self::ContentDisabled,
        Self::ContentOnAccent,
        Self::ContentLink,
        Self::BorderSubtle,
        Self::BorderDefault,
        Self::BorderStrong,
        Self::BorderHover,
        Self::BorderFocused,
        Self::BorderDisabled,
        Self::BorderInvalid,
        Self::SelectionBackground,
        Self::SelectionForeground,
        Self::FocusIndicator,
        Self::FocusSeparator,
        Self::FocusRing,
        Self::OverlayScrim,
        Self::AccentSubtle,
        Self::AccentDefault,
        Self::AccentHover,
        Self::AccentPressed,
        Self::AccentFocus,
        Self::AccentForeground,
        Self::StatusInfoForeground,
        Self::StatusInfoSurface,
        Self::StatusInfoBorder,
        Self::StatusInfoStrong,
        Self::StatusSuccessForeground,
        Self::StatusSuccessSurface,
        Self::StatusSuccessBorder,
        Self::StatusSuccessStrong,
        Self::StatusWarningForeground,
        Self::StatusWarningSurface,
        Self::StatusWarningBorder,
        Self::StatusWarningStrong,
        Self::StatusDangerForeground,
        Self::StatusDangerSurface,
        Self::StatusDangerBorder,
        Self::StatusDangerStrong,
    ];
}

/// Surface color tokens.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceColors {
    /// Main application shell.
    pub application: Color,
    /// Workspace or viewport canvas.
    pub workspace: Color,
    /// Passive panel surface.
    pub panel: Color,
    /// Elevated panel surface.
    pub panel_raised: Color,
    /// General raised surface.
    pub raised: Color,
    /// Ordinary control surface.
    pub control: Color,
    /// Hovered control surface.
    pub control_hover: Color,
    /// Pressed control surface.
    pub control_pressed: Color,
    /// Disabled control surface.
    pub control_disabled: Color,
    /// Floating overlay surface.
    pub overlay: Color,
    /// General hovered surface.
    pub hover: Color,
    /// Sunken input or collection surface.
    pub sunken: Color,
}

/// Content color tokens.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContentColors {
    /// Primary content.
    pub primary: Color,
    /// Secondary content.
    pub secondary: Color,
    /// Muted content.
    pub muted: Color,
    /// Disabled content.
    pub disabled: Color,
    /// Content painted on an accent or strong status surface.
    pub on_accent: Color,
    /// Link content.
    pub link: Color,
}

/// Border color tokens.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderColors {
    /// Subtle separator or outline.
    pub subtle: Color,
    /// Default outline.
    pub default: Color,
    /// Strong outline.
    pub strong: Color,
    /// Hovered outline.
    pub hover: Color,
    /// Focused outline.
    pub focused: Color,
    /// Disabled outline.
    pub disabled: Color,
    /// Invalid-value outline.
    pub invalid: Color,
}

/// Selection color tokens.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionColors {
    /// Selection background.
    pub background: Color,
    /// Content painted on a selection background.
    pub foreground: Color,
}

/// Focus color tokens.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusColors {
    /// Focus indicator.
    pub indicator: Color,
    /// Focus separator.
    pub separator: Color,
    /// Focus ring.
    pub ring: Color,
}

/// Overlay color tokens.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayColors {
    /// Modal or blocking scrim.
    pub scrim: Color,
}

/// Accent color tokens.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccentColors {
    /// Subtle accent surface.
    pub subtle: Color,
    /// Default accent.
    pub default: Color,
    /// Hovered accent.
    pub hover: Color,
    /// Pressed accent.
    pub pressed: Color,
    /// Accent focus color.
    pub focus: Color,
    /// Content painted on an accent surface.
    pub foreground: Color,
}

/// One stored status color family.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusColorFamilyColors {
    /// Status content.
    pub foreground: Color,
    /// Subtle status surface.
    pub surface: Color,
    /// Status outline.
    pub border: Color,
    /// Strong status accent.
    pub strong: Color,
}

/// Stored status color families.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusColors {
    /// Informational colors.
    pub info: StatusColorFamilyColors,
    /// Success colors.
    pub success: StatusColorFamilyColors,
    /// Warning colors.
    pub warning: StatusColorFamilyColors,
    /// Danger colors.
    pub danger: StatusColorFamilyColors,
}

/// Grouped semantic theme color tokens.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColors {
    /// Surface colors.
    pub surface: SurfaceColors,
    /// Content colors.
    pub content: ContentColors,
    /// Border colors.
    pub border: BorderColors,
    /// Selection colors.
    pub selection: SelectionColors,
    /// Focus colors.
    pub focus: FocusColors,
    /// Overlay colors.
    pub overlay: OverlayColors,
    /// Accent colors.
    pub accent: AccentColors,
    /// Status colors.
    pub status: StatusColors,
}

impl ThemeColors {
    /// Returns the exact normative dark semantic palette.
    #[must_use]
    pub const fn default_dark() -> Self {
        Self {
            surface: SurfaceColors {
                application: Color::rgb8(0x11, 0x11, 0x11),
                workspace: Color::rgb8(0x0B, 0x0B, 0x0B),
                panel: Color::rgb8(0x14, 0x14, 0x14),
                panel_raised: Color::rgb8(0x18, 0x18, 0x18),
                raised: Color::rgb8(0x18, 0x18, 0x18),
                control: Color::rgb8(0x18, 0x18, 0x18),
                control_hover: Color::rgb8(0x1C, 0x1C, 0x1C),
                control_pressed: Color::rgb8(0x2A, 0x2A, 0x2A),
                control_disabled: Color::rgb8(0x14, 0x14, 0x14),
                overlay: Color::rgb8(0x18, 0x18, 0x18),
                hover: Color::rgb8(0x1C, 0x1C, 0x1C),
                sunken: Color::rgb8(0x0B, 0x0B, 0x0B),
            },
            content: ContentColors {
                primary: Color::rgb8(0xE8, 0xE8, 0xE8),
                secondary: Color::rgb8(0xB8, 0xB8, 0xB8),
                muted: Color::rgb8(0x99, 0x99, 0x99),
                disabled: Color::rgb8(0x66, 0x66, 0x66),
                on_accent: Color::rgb8(0xFF, 0xFF, 0xFF),
                link: Color::rgb8(0x25, 0x9C, 0xF0),
            },
            border: BorderColors {
                subtle: Color::rgb8(0x22, 0x22, 0x22),
                default: Color::rgb8(0x2A, 0x2A, 0x2A),
                strong: Color::rgb8(0x3D, 0x3D, 0x3D),
                hover: Color::rgb8(0x3D, 0x3D, 0x3D),
                focused: Color::rgb8(0x4D, 0xB2, 0xFF),
                disabled: Color::rgb8(0x22, 0x22, 0x22),
                invalid: Color::rgb8(0xF1, 0x8A, 0x90),
            },
            selection: SelectionColors {
                background: Color::rgb8(0x0C, 0x8C, 0xE9),
                foreground: Color::rgb8(0xFF, 0xFF, 0xFF),
            },
            focus: FocusColors {
                indicator: Color::rgb8(0x4D, 0xB2, 0xFF),
                separator: Color::rgb8(0x0B, 0x0B, 0x0B),
                ring: Color::rgb8(0x4D, 0xB2, 0xFF),
            },
            overlay: OverlayColors {
                scrim: Color::rgb8(0x0B, 0x0B, 0x0B),
            },
            accent: AccentColors {
                subtle: Color::rgb8(0x0B, 0x2A, 0x3F),
                default: Color::rgb8(0x0C, 0x8C, 0xE9),
                hover: Color::rgb8(0x25, 0x9C, 0xF0),
                pressed: Color::rgb8(0x08, 0x76, 0xC5),
                focus: Color::rgb8(0x4D, 0xB2, 0xFF),
                foreground: Color::rgb8(0xFF, 0xFF, 0xFF),
            },
            status: StatusColors {
                info: StatusColorFamilyColors {
                    foreground: Color::rgb8(0x6C, 0xBF, 0xFF),
                    surface: Color::rgb8(0x10, 0x18, 0x20),
                    border: Color::rgb8(0x25, 0x34, 0x3F),
                    strong: Color::rgb8(0x0C, 0x8C, 0xE9),
                },
                success: StatusColorFamilyColors {
                    foreground: Color::rgb8(0x72, 0xD9, 0x98),
                    surface: Color::rgb8(0x12, 0x1A, 0x15),
                    border: Color::rgb8(0x29, 0x37, 0x2E),
                    strong: Color::rgb8(0x39, 0xB8, 0x68),
                },
                warning: StatusColorFamilyColors {
                    foreground: Color::rgb8(0xF0, 0xC6, 0x6D),
                    surface: Color::rgb8(0x1A, 0x17, 0x11),
                    border: Color::rgb8(0x3A, 0x33, 0x26),
                    strong: Color::rgb8(0xD9, 0xA4, 0x41),
                },
                danger: StatusColorFamilyColors {
                    foreground: Color::rgb8(0xF1, 0x8A, 0x90),
                    surface: Color::rgb8(0x1B, 0x13, 0x14),
                    border: Color::rgb8(0x3D, 0x29, 0x2B),
                    strong: Color::rgb8(0xD9, 0x53, 0x5B),
                },
            },
        }
    }

    /// Returns a semantic color.
    #[must_use]
    pub const fn get(self, role: SemanticColor) -> Color {
        match role {
            SemanticColor::SurfaceApplication => self.surface.application,
            SemanticColor::SurfaceWorkspace => self.surface.workspace,
            SemanticColor::SurfacePanel => self.surface.panel,
            SemanticColor::SurfacePanelRaised => self.surface.panel_raised,
            SemanticColor::SurfaceRaised => self.surface.raised,
            SemanticColor::SurfaceControl => self.surface.control,
            SemanticColor::SurfaceControlHover => self.surface.control_hover,
            SemanticColor::SurfaceControlPressed => self.surface.control_pressed,
            SemanticColor::SurfaceControlDisabled => self.surface.control_disabled,
            SemanticColor::SurfaceOverlay => self.surface.overlay,
            SemanticColor::SurfaceHover => self.surface.hover,
            SemanticColor::SurfaceSunken => self.surface.sunken,
            SemanticColor::ContentPrimary => self.content.primary,
            SemanticColor::ContentSecondary => self.content.secondary,
            SemanticColor::ContentMuted => self.content.muted,
            SemanticColor::ContentDisabled => self.content.disabled,
            SemanticColor::ContentOnAccent => self.content.on_accent,
            SemanticColor::ContentLink => self.content.link,
            SemanticColor::BorderSubtle => self.border.subtle,
            SemanticColor::BorderDefault => self.border.default,
            SemanticColor::BorderStrong => self.border.strong,
            SemanticColor::BorderHover => self.border.hover,
            SemanticColor::BorderFocused => self.border.focused,
            SemanticColor::BorderDisabled => self.border.disabled,
            SemanticColor::BorderInvalid => self.border.invalid,
            SemanticColor::SelectionBackground => self.selection.background,
            SemanticColor::SelectionForeground => self.selection.foreground,
            SemanticColor::FocusIndicator => self.focus.indicator,
            SemanticColor::FocusSeparator => self.focus.separator,
            SemanticColor::FocusRing => self.focus.ring,
            SemanticColor::OverlayScrim => self.overlay.scrim,
            SemanticColor::AccentSubtle => self.accent.subtle,
            SemanticColor::AccentDefault => self.accent.default,
            SemanticColor::AccentHover => self.accent.hover,
            SemanticColor::AccentPressed => self.accent.pressed,
            SemanticColor::AccentFocus => self.accent.focus,
            SemanticColor::AccentForeground => self.accent.foreground,
            SemanticColor::StatusInfoForeground => self.status.info.foreground,
            SemanticColor::StatusInfoSurface => self.status.info.surface,
            SemanticColor::StatusInfoBorder => self.status.info.border,
            SemanticColor::StatusInfoStrong => self.status.info.strong,
            SemanticColor::StatusSuccessForeground => self.status.success.foreground,
            SemanticColor::StatusSuccessSurface => self.status.success.surface,
            SemanticColor::StatusSuccessBorder => self.status.success.border,
            SemanticColor::StatusSuccessStrong => self.status.success.strong,
            SemanticColor::StatusWarningForeground => self.status.warning.foreground,
            SemanticColor::StatusWarningSurface => self.status.warning.surface,
            SemanticColor::StatusWarningBorder => self.status.warning.border,
            SemanticColor::StatusWarningStrong => self.status.warning.strong,
            SemanticColor::StatusDangerForeground => self.status.danger.foreground,
            SemanticColor::StatusDangerSurface => self.status.danger.surface,
            SemanticColor::StatusDangerBorder => self.status.danger.border,
            SemanticColor::StatusDangerStrong => self.status.danger.strong,
        }
    }
}

/// Exact compact spacing-step identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpacingStep {
    /// `spacing.0`.
    Zero,
    /// `spacing.1`.
    One,
    /// `spacing.2`.
    Two,
    /// `spacing.3`.
    Three,
    /// `spacing.4`.
    Four,
    /// `spacing.5`.
    Five,
    /// `spacing.6`.
    Six,
    /// `spacing.7`.
    Seven,
    /// `spacing.8`.
    Eight,
}

impl SpacingStep {
    /// Every spacing step in ascending ladder order.
    pub const ALL: &'static [Self] = &[
        Self::Zero,
        Self::One,
        Self::Two,
        Self::Three,
        Self::Four,
        Self::Five,
        Self::Six,
        Self::Seven,
        Self::Eight,
    ];
}

/// Semantic spacing role resolved from the configured compact ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpacingRole {
    /// Gap between an icon and its label.
    IconLabelGap,
    /// Gap between tightly grouped controls.
    TightControlGap,
    /// Inline padding for compact controls.
    CompactInlineControlPadding,
    /// Inline padding for default controls.
    DefaultInlineControlPadding,
    /// Block-axis padding for controls.
    BlockControlPadding,
    /// Gap between inspector labels and values.
    InspectorLabelValueGap,
    /// Gap between ordinary groups.
    GroupGap,
    /// Panel content inset.
    PanelPadding,
    /// Gap between sections.
    SectionGap,
}

impl SpacingRole {
    /// Every semantic spacing role in normative specification order.
    pub const ALL: &'static [Self] = &[
        Self::IconLabelGap,
        Self::TightControlGap,
        Self::CompactInlineControlPadding,
        Self::DefaultInlineControlPadding,
        Self::BlockControlPadding,
        Self::InspectorLabelValueGap,
        Self::GroupGap,
        Self::PanelPadding,
        Self::SectionGap,
    ];

    /// Returns the compact ladder step that supplies this role.
    #[must_use]
    pub const fn step(self) -> SpacingStep {
        match self {
            Self::IconLabelGap | Self::TightControlGap | Self::BlockControlPadding => {
                SpacingStep::Two
            }
            Self::CompactInlineControlPadding => SpacingStep::Three,
            Self::DefaultInlineControlPadding
            | Self::InspectorLabelValueGap
            | Self::GroupGap
            | Self::PanelPadding => SpacingStep::Four,
            Self::SectionGap => SpacingStep::Six,
        }
    }
}

/// Exact nine-step compact spacing token scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingScale {
    /// `spacing.0`.
    pub zero: f32,
    /// `spacing.1`.
    pub one: f32,
    /// `spacing.2`.
    pub two: f32,
    /// `spacing.3`.
    pub three: f32,
    /// `spacing.4`.
    pub four: f32,
    /// `spacing.5`.
    pub five: f32,
    /// `spacing.6`.
    pub six: f32,
    /// `spacing.7`.
    pub seven: f32,
    /// `spacing.8`.
    pub eight: f32,
}

impl SpacingScale {
    /// Creates a spacing scale in ascending step order.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        zero: f32,
        one: f32,
        two: f32,
        three: f32,
        four: f32,
        five: f32,
        six: f32,
        seven: f32,
        eight: f32,
    ) -> Self {
        Self {
            zero,
            one,
            two,
            three,
            four,
            five,
            six,
            seven,
            eight,
        }
    }

    /// Resolves the configured value for an exact spacing step.
    #[must_use]
    pub const fn get(self, step: SpacingStep) -> f32 {
        match step {
            SpacingStep::Zero => self.zero,
            SpacingStep::One => self.one,
            SpacingStep::Two => self.two,
            SpacingStep::Three => self.three,
            SpacingStep::Four => self.four,
            SpacingStep::Five => self.five,
            SpacingStep::Six => self.six,
            SpacingStep::Seven => self.seven,
            SpacingStep::Eight => self.eight,
        }
    }

    /// Resolves a semantic spacing role through its configured ladder step.
    #[must_use]
    pub const fn resolve(self, role: SpacingRole) -> f32 {
        self.get(role.step())
    }
}

/// Exact control-height size tokens in logical units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlSizeScale {
    /// Extra-small control height.
    pub xs: f32,
    /// Small control height.
    pub sm: f32,
    /// Medium control height.
    pub md: f32,
    /// Large control height.
    pub lg: f32,
}

impl ControlSizeScale {
    /// Creates a control size scale in ascending size order.
    #[must_use]
    pub const fn new(xs: f32, sm: f32, md: f32, lg: f32) -> Self {
        Self { xs, sm, md, lg }
    }
}

/// Exact row-height size tokens in logical units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowSizeScale {
    /// Compact collection row height.
    pub compact: f32,
    /// Standard collection row height.
    pub standard: f32,
}

impl RowSizeScale {
    /// Creates a row size scale in compact and standard order.
    #[must_use]
    pub const fn new(compact: f32, standard: f32) -> Self {
        Self { compact, standard }
    }
}

/// Exact icon side-length size tokens in logical units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconSizeScale {
    /// Small icon side length.
    pub sm: f32,
    /// Medium icon side length.
    pub md: f32,
    /// Large icon side length.
    pub lg: f32,
}

impl IconSizeScale {
    /// Creates an icon size scale in ascending size order.
    #[must_use]
    pub const fn new(sm: f32, md: f32, lg: f32) -> Self {
        Self { sm, md, lg }
    }
}

/// Exact handle visual and interaction size tokens in logical units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandleSizeScale {
    /// Visible handle thickness.
    pub visual: f32,
    /// Non-overlapping handle hit target.
    pub hit: f32,
}

impl HandleSizeScale {
    /// Creates a handle size scale in visual and hit-target order.
    #[must_use]
    pub const fn new(visual: f32, hit: f32) -> Self {
        Self { visual, hit }
    }
}

/// Typed identity for every exact size token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SizeToken {
    /// `size.control.xs`.
    ControlXs,
    /// `size.control.sm`.
    ControlSm,
    /// `size.control.md`.
    ControlMd,
    /// `size.control.lg`.
    ControlLg,
    /// `size.row.compact`.
    RowCompact,
    /// `size.row.standard`.
    RowStandard,
    /// `size.tab`.
    Tab,
    /// `size.panelHeader`.
    PanelHeader,
    /// `size.workspaceBar`.
    WorkspaceBar,
    /// `size.icon.sm`.
    IconSm,
    /// `size.icon.md`.
    IconMd,
    /// `size.icon.lg`.
    IconLg,
    /// `size.handle.visual`.
    HandleVisual,
    /// `size.handle.hit`.
    HandleHit,
}

impl SizeToken {
    /// Every exact size token in normative grouped-field order.
    pub const ALL: &'static [Self] = &[
        Self::ControlXs,
        Self::ControlSm,
        Self::ControlMd,
        Self::ControlLg,
        Self::RowCompact,
        Self::RowStandard,
        Self::Tab,
        Self::PanelHeader,
        Self::WorkspaceBar,
        Self::IconSm,
        Self::IconMd,
        Self::IconLg,
        Self::HandleVisual,
        Self::HandleHit,
    ];
}

/// Exact grouped size-token foundation in logical units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeScale {
    /// Control heights.
    pub control: ControlSizeScale,
    /// Collection row heights.
    pub row: RowSizeScale,
    /// Tab height.
    pub tab: f32,
    /// Panel-header height.
    pub panel_header: f32,
    /// Workspace-bar height.
    pub workspace_bar: f32,
    /// Icon side lengths.
    pub icon: IconSizeScale,
    /// Handle visual and interaction sizes.
    pub handle: HandleSizeScale,
}

impl SizeScale {
    /// Creates the grouped size foundation in stored field order.
    #[must_use]
    pub const fn new(
        control: ControlSizeScale,
        row: RowSizeScale,
        tab: f32,
        panel_header: f32,
        workspace_bar: f32,
        icon: IconSizeScale,
        handle: HandleSizeScale,
    ) -> Self {
        Self {
            control,
            row,
            tab,
            panel_header,
            workspace_bar,
            icon,
            handle,
        }
    }

    /// Resolves the configured value for an exact size token.
    #[must_use]
    pub const fn get(self, token: SizeToken) -> f32 {
        match token {
            SizeToken::ControlXs => self.control.xs,
            SizeToken::ControlSm => self.control.sm,
            SizeToken::ControlMd => self.control.md,
            SizeToken::ControlLg => self.control.lg,
            SizeToken::RowCompact => self.row.compact,
            SizeToken::RowStandard => self.row.standard,
            SizeToken::Tab => self.tab,
            SizeToken::PanelHeader => self.panel_header,
            SizeToken::WorkspaceBar => self.workspace_bar,
            SizeToken::IconSm => self.icon.sm,
            SizeToken::IconMd => self.icon.md,
            SizeToken::IconLg => self.icon.lg,
            SizeToken::HandleVisual => self.handle.visual,
            SizeToken::HandleHit => self.handle.hit,
        }
    }
}

/// Radius token scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusScale {
    /// No rounding.
    pub none: CornerRadius,
    /// Small corner radius for ordinary controls and compact selections.
    pub sm: CornerRadius,
    /// Medium corner radius for menus, dropdowns, nodes, and popovers.
    pub md: CornerRadius,
    /// Large corner radius for dialogs and prominent floating surfaces.
    pub lg: CornerRadius,
    /// Fully rounded radius for dots, circular handles, and deliberate pills.
    pub full: CornerRadius,
}

impl RadiusScale {
    /// Creates an equal-corner radius scale with `none` fixed at zero.
    #[must_use]
    pub const fn from_values(sm: f32, md: f32, lg: f32, full: f32) -> Self {
        Self {
            none: CornerRadius::all(0.0),
            sm: CornerRadius::all(sm),
            md: CornerRadius::all(md),
            lg: CornerRadius::all(lg),
            full: CornerRadius::all(full),
        }
    }
}

/// Font and line metrics for a text role.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontToken {
    /// Font family name or logical family.
    pub family: &'static str,
    /// Font size in logical units.
    pub size: f32,
    /// Line height in logical units.
    pub line_height: f32,
}

impl FontToken {
    /// Creates a font token.
    #[must_use]
    pub const fn new(family: &'static str, size: f32, line_height: f32) -> Self {
        Self {
            family,
            size,
            line_height,
        }
    }
}

/// Semantic text style role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextRole {
    /// Body copy and ordinary labels.
    Body,
    /// Compact labels inside controls.
    Label,
    /// Secondary captions.
    Caption,
    /// Section or panel headings.
    Title,
    /// Monospace values and code-like labels.
    Monospace,
}

/// Typography token scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypographyScale {
    /// Body copy and ordinary labels.
    pub body: FontToken,
    /// Compact labels inside controls.
    pub label: FontToken,
    /// Secondary captions.
    pub caption: FontToken,
    /// Section or panel headings.
    pub title: FontToken,
    /// Monospace values and code-like labels.
    pub monospace: FontToken,
}

impl TypographyScale {
    /// Returns a font token for a text role.
    #[must_use]
    pub const fn get(self, role: TextRole) -> FontToken {
        match role {
            TextRole::Body => self.body,
            TextRole::Label => self.label,
            TextRole::Caption => self.caption,
            TextRole::Title => self.title,
            TextRole::Monospace => self.monospace,
        }
    }
}

/// Opacity tokens for state overlays and disabled affordances.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpacityScale {
    /// Disabled content opacity.
    pub disabled: f32,
    /// Hover overlay opacity.
    pub hover: f32,
    /// Pressed overlay opacity.
    pub pressed: f32,
    /// Selection fill opacity.
    pub selection: f32,
    /// Modal or menu scrim opacity.
    pub overlay_scrim: f32,
}

/// Semantic elevation identity for surface layering and shadow selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElevationLevel {
    /// Docked panels and ordinary controls.
    None,
    /// Tooltips and small floating affordances.
    Low,
    /// Dropdowns, menus, and popovers.
    Medium,
    /// Dialogs, command palettes, and modal capture layers.
    High,
}

/// Elevation values for the four semantic surface levels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevationScale {
    /// Docked panels and ordinary controls.
    pub none: f32,
    /// Tooltips and small floating affordances.
    pub low: f32,
    /// Dropdowns, menus, and popovers.
    pub medium: f32,
    /// Dialogs, command palettes, and modal capture layers.
    pub high: f32,
}

impl ElevationScale {
    /// Creates an elevation scale in semantic level order.
    #[must_use]
    pub const fn new(none: f32, low: f32, medium: f32, high: f32) -> Self {
        Self {
            none,
            low,
            medium,
            high,
        }
    }

    /// Resolves the configured value for a typed elevation level.
    #[must_use]
    pub const fn get(self, level: ElevationLevel) -> f32 {
        match level {
            ElevationLevel::None => self.none,
            ElevationLevel::Low => self.low,
            ElevationLevel::Medium => self.medium,
            ElevationLevel::High => self.high,
        }
    }
}

/// Renderer-neutral shadow style for an elevated surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowRecipe {
    /// Shadow offset in logical units.
    pub offset: Vec2,
    /// Gaussian blur radius in logical units.
    pub blur_radius: f32,
    /// Amount to expand or shrink the source rectangle before blurring.
    pub spread: f32,
    /// Uniform corner radius for the shadow shape.
    pub radius: f32,
    /// Shadow color.
    pub color: Color,
}

impl ShadowRecipe {
    /// Creates a shadow primitive for a rectangle.
    #[must_use]
    pub const fn primitive(self, rect: Rect) -> ShadowPrimitive {
        ShadowPrimitive::new(
            rect,
            self.offset,
            self.blur_radius,
            self.spread,
            self.radius,
            self.color,
        )
    }
}

/// Motion duration tokens in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DurationScale {
    /// Immediate transition.
    pub instant: f32,
    /// Fast affordance transition.
    pub fast: f32,
    /// Ordinary transition.
    pub normal: f32,
    /// Deliberate transition.
    pub slow: f32,
}

/// Stroke widths reserved for the two-layer focus treatment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusStrokeScale {
    /// Primary focus indication width.
    pub primary: f32,
    /// Contrast separator width between focus and component paint.
    pub separator: f32,
}

/// Shared stroke-width foundation roles in logical units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeScale {
    /// One-unit structural boundary and divider width.
    pub hairline: f32,
    /// Ordinary control and surface boundary width.
    pub default: f32,
    /// Strong indicator and emphasis width.
    pub emphasis: f32,
    /// Widths reserved for the two-layer focus treatment.
    pub focus: FocusStrokeScale,
}

impl StrokeScale {
    /// Creates a stroke scale in hairline, default, emphasis, focus-primary,
    /// and focus-separator order.
    #[must_use]
    pub const fn from_values(
        hairline: f32,
        default: f32,
        emphasis: f32,
        focus_primary: f32,
        focus_separator: f32,
    ) -> Self {
        Self {
            hairline,
            default,
            emphasis,
            focus: FocusStrokeScale {
                primary: focus_primary,
                separator: focus_separator,
            },
        }
    }
}

/// Control sizing and padding metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlMetrics {
    /// Default one-line control height.
    pub control_height: f32,
    /// Compact control height.
    pub compact_control_height: f32,
    /// Checkbox and radio side length.
    pub check_size: f32,
    /// Horizontal text/control padding.
    pub padding_x: f32,
    /// Vertical text/control padding.
    pub padding_y: f32,
}
