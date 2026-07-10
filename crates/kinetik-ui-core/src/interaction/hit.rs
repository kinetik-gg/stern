use crate::{Point, Rect, Transform, UiInput, UiMemory, WidgetId};

/// Returns true when pointer input is inside a rectangle.
#[must_use]
pub fn hit_test(rect: Rect, input: &UiInput) -> bool {
    input
        .pointer
        .position
        .is_some_and(|position| rect.contains_point(position))
}

/// Returns true when pointer input is inside a rectangle after applying an inverse transform.
#[must_use]
pub fn hit_test_transformed(rect: Rect, local_to_screen: Transform, input: &UiInput) -> bool {
    input.pointer.position.is_some_and(|position| {
        local_to_screen
            .try_inverse()
            .is_some_and(|screen_to_local| {
                let local_position = screen_to_local.transform_point(position);
                point_is_finite(local_position) && rect.contains_point(local_position)
            })
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum HitTarget {
    Rect,
    Transformed(Transform),
}

impl HitTarget {
    pub(super) fn hit_test(self, rect: Rect, input: &UiInput) -> bool {
        match self {
            Self::Rect => hit_test(rect, input),
            Self::Transformed(local_to_screen) => {
                hit_test_transformed(rect, local_to_screen, input)
            }
        }
    }

    pub(super) fn routed_hit_test(
        self,
        id: WidgetId,
        rect: Rect,
        input: &UiInput,
        memory: &UiMemory,
    ) -> bool {
        memory.pointer_route_allows(id) && self.hit_test(rect, input)
    }
}

fn point_is_finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}
