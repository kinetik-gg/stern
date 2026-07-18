use stern_core::{
    Point, PointerOrder, PointerTarget, PointerTargetPlan, Rect, StaticIcon, WidgetId,
};

/// Retained titlebar trigger for the platform-owned window system menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowSystemMenuTrigger {
    id: WidgetId,
    titlebar_rect: Rect,
    icon: StaticIcon,
    request_position: Point,
}

impl WindowSystemMenuTrigger {
    /// Creates a trigger from exact window-local logical geometry.
    #[must_use]
    pub fn new(
        id: WidgetId,
        titlebar_rect: Rect,
        icon: impl Into<StaticIcon>,
        request_position: Point,
    ) -> Self {
        Self {
            id,
            titlebar_rect,
            icon: icon.into(),
            request_position,
        }
    }

    /// Returns the stable widget identity owned by this trigger.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        self.id
    }

    /// Returns the exact titlebar rectangle used for paint and hit testing.
    #[must_use]
    pub const fn titlebar_rect(&self) -> Rect {
        self.titlebar_rect
    }

    /// Returns the static icon presented by the trigger.
    #[must_use]
    pub const fn icon(&self) -> StaticIcon {
        self.icon
    }

    /// Returns the unchanged window-local logical system-menu position.
    #[must_use]
    pub const fn request_position(&self) -> Point {
        self.request_position
    }

    /// Declares the trigger's exact pointer target when all geometry is valid.
    pub fn declare_pointer_target(
        &self,
        plan: &mut PointerTargetPlan,
        order: PointerOrder,
    ) -> bool {
        if !self.is_valid() {
            return false;
        }
        plan.target(PointerTarget::new(self.id, self.titlebar_rect, order));
        true
    }

    pub(crate) fn is_valid(&self) -> bool {
        let rect = self.titlebar_rect;
        rect.x.is_finite()
            && rect.y.is_finite()
            && rect.width.is_finite()
            && rect.height.is_finite()
            && !rect.is_empty()
            && rect.max_x().is_finite()
            && rect.max_y().is_finite()
            && self.request_position.x.is_finite()
            && self.request_position.y.is_finite()
    }
}
