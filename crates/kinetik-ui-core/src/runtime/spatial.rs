use crate::{
    ClipId, InputWheelDelta, LayerId, MouseButton, Point, PointerButtonState, Primitive, Rect,
    Transform, UiInput, UiInputEvent, Vec2,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct SpatialStack {
    state: SpatialState,
    scopes: Vec<SpatialScope>,
}

pub(crate) struct LocalizedInput {
    pub input: UiInput,
    pub event_ordinals: Vec<usize>,
    pub cleanup_only: Vec<bool>,
}

impl SpatialStack {
    pub(crate) fn observe_primitive(&mut self, primitive: &Primitive) {
        match primitive {
            Primitive::ClipBegin { id, rect } => self.push_clip(*id, *rect),
            Primitive::ClipEnd { id } => self.pop_clip(*id),
            Primitive::LayerBegin { id } => self.scopes.push(SpatialScope::Layer(*id)),
            Primitive::LayerEnd { id } => self.pop_layer(*id),
            Primitive::TransformBegin(transform) => self.push_transform(*transform),
            Primitive::TransformEnd => self.pop_transform(),
            Primitive::Rect(_)
            | Primitive::Line(_)
            | Primitive::Shadow(_)
            | Primitive::Path(_)
            | Primitive::Text(_)
            | Primitive::Image(_)
            | Primitive::Texture(_) => {}
        }
    }

    pub(crate) fn localize_input(
        &self,
        root: &UiInput,
        preserve_primary_release: bool,
        preserve_secondary_release: bool,
        root_input_conflict: bool,
    ) -> LocalizedInput {
        let mut input = root.clone();
        let local_position = root.pointer.position.and_then(|position| {
            if !self.accepts_screen_point(position) {
                return None;
            }
            let inverse = self.state.screen_to_local?;
            let local = inverse.transform_point(position);
            point_is_finite(local).then_some(local)
        });
        input.pointer.position = local_position;

        let vectors_are_visible = self.state.screen_to_local.is_some()
            && root.pointer.position.map_or_else(
                || self.state.clips.is_empty(),
                |position| self.accepts_screen_point(position),
            );
        if vectors_are_visible {
            input.pointer.delta = self
                .state
                .screen_to_local
                .and_then(|inverse| transform_vector(inverse, root.pointer.delta))
                .unwrap_or(Vec2::ZERO);
            input.pointer.wheel_delta = self
                .state
                .screen_to_local
                .and_then(|inverse| transform_vector(inverse, root.pointer.wheel_delta))
                .unwrap_or(Vec2::ZERO);
        } else {
            input.pointer.delta = Vec2::ZERO;
            input.pointer.wheel_delta = Vec2::ZERO;
            input.pointer.primary =
                release_cleanup(input.pointer.primary, preserve_primary_release);
            input.pointer.secondary =
                release_cleanup(input.pointer.secondary, preserve_secondary_release);
            input.pointer.middle = PointerButtonState::default();
            input.pointer.other_buttons.clear();
            input.pointer.click_count = 0;
        }

        if root.events.is_empty() {
            return LocalizedInput {
                input,
                event_ordinals: Vec::new(),
                cleanup_only: Vec::new(),
            };
        }

        let (events, event_ordinals, cleanup_only) =
            self.localize_events(root, preserve_primary_release, preserve_secondary_release);
        input.events = events;
        if root_input_conflict {
            return LocalizedInput {
                input,
                event_ordinals,
                cleanup_only,
            };
        }

        let release_snapshot = input.pointer.clone();
        input.pointer.begin_frame();
        let mut saw_release_all = false;
        for event in &input.events {
            match event {
                UiInputEvent::PointerMoved { delta, .. } => {
                    input.pointer.delta = add_vectors(input.pointer.delta, *delta);
                }
                UiInputEvent::PointerLeft => input.pointer.delta = Vec2::ZERO,
                UiInputEvent::PointerButton {
                    button,
                    down,
                    click_count,
                    ..
                } => {
                    input.pointer.record_button_edge(*button, *down);
                    input.pointer.click_count = *click_count;
                }
                UiInputEvent::PointerReleaseAll { .. } => {
                    saw_release_all = true;
                    input.pointer.mark_release_all_cancelled();
                }
                UiInputEvent::Wheel { delta, .. } => {
                    input.pointer.wheel_delta =
                        add_vectors(input.pointer.wheel_delta, delta.value());
                }
                _ => {}
            }
        }
        if saw_release_all {
            restore_release_edges(
                &mut input.pointer,
                &release_snapshot,
                preserve_primary_release,
                preserve_secondary_release,
            );
        }
        LocalizedInput {
            input,
            event_ordinals,
            cleanup_only,
        }
    }

    fn localize_events(
        &self,
        root: &UiInput,
        preserve_primary_release: bool,
        preserve_secondary_release: bool,
    ) -> (Vec<UiInputEvent>, Vec<usize>, Vec<bool>) {
        let mut localized = Vec::new();
        let mut primary_cleanup_required = preserve_primary_release;
        let mut secondary_cleanup_required = preserve_secondary_release;

        for (ordinal, event) in root.events.iter().enumerate() {
            let ordinary = match event {
                UiInputEvent::PointerButton { position, .. }
                | UiInputEvent::PointerReleaseAll { position } => {
                    self.ordinary_event_allowed(*position)
                }
                _ => true,
            };
            let cleanup_only = match event {
                UiInputEvent::PointerButton {
                    button,
                    down: false,
                    ..
                } => {
                    let required = (*button == MouseButton::Primary && primary_cleanup_required)
                        || (*button == MouseButton::Secondary && secondary_cleanup_required);
                    required && !ordinary
                }
                UiInputEvent::PointerReleaseAll { .. } => {
                    (primary_cleanup_required || secondary_cleanup_required) && !ordinary
                }
                _ => false,
            };

            if let Some(local_event) =
                self.localize_event(event, primary_cleanup_required, secondary_cleanup_required)
            {
                localized.push((ordinal, local_event, cleanup_only));
            }

            match event {
                UiInputEvent::PointerButton {
                    button, down: true, ..
                } if ordinary => match button {
                    MouseButton::Primary => primary_cleanup_required = true,
                    MouseButton::Secondary => secondary_cleanup_required = true,
                    MouseButton::Middle | MouseButton::Other(_) => {}
                },
                UiInputEvent::PointerButton {
                    button,
                    down: false,
                    ..
                } => match button {
                    MouseButton::Primary => primary_cleanup_required = false,
                    MouseButton::Secondary => secondary_cleanup_required = false,
                    MouseButton::Middle | MouseButton::Other(_) => {}
                },
                UiInputEvent::PointerReleaseAll { .. }
                | UiInputEvent::WindowFocusChanged(false) => {
                    primary_cleanup_required = false;
                    secondary_cleanup_required = false;
                }
                _ => {}
            }
        }
        let ordinals = localized.iter().map(|(ordinal, _, _)| *ordinal).collect();
        let cleanup_only = localized
            .iter()
            .map(|(_, _, cleanup_only)| *cleanup_only)
            .collect();
        let events = localized.into_iter().map(|(_, event, _)| event).collect();
        (events, ordinals, cleanup_only)
    }

    fn localize_event(
        &self,
        event: &UiInputEvent,
        preserve_primary_release: bool,
        preserve_secondary_release: bool,
    ) -> Option<UiInputEvent> {
        match event {
            UiInputEvent::PointerMoved { position, delta } => {
                let position = self.localize_ordinary_position(*position)?;
                let delta = self.transform_event_vector(*delta)?;
                Some(UiInputEvent::PointerMoved { position, delta })
            }
            UiInputEvent::PointerLeft => Some(UiInputEvent::PointerLeft),
            UiInputEvent::PointerButton {
                button,
                down,
                click_count,
                position,
            } => {
                let ordinary = self.ordinary_event_allowed(*position);
                let cleanup = !*down
                    && ((*button == MouseButton::Primary && preserve_primary_release)
                        || (*button == MouseButton::Secondary && preserve_secondary_release));
                if !ordinary && !cleanup {
                    return None;
                }
                Some(UiInputEvent::PointerButton {
                    button: *button,
                    down: *down,
                    click_count: *click_count,
                    position: self.transform_optional_position(*position),
                })
            }
            UiInputEvent::PointerReleaseAll { position } => Some(UiInputEvent::PointerReleaseAll {
                position: self.transform_optional_position(*position),
            }),
            UiInputEvent::Wheel { delta, position } => {
                if !self.ordinary_event_allowed(*position) {
                    return None;
                }
                let delta = match *delta {
                    InputWheelDelta::Lines(delta) => InputWheelDelta::Lines(delta),
                    InputWheelDelta::Pixels(delta) => {
                        InputWheelDelta::Pixels(self.transform_event_vector(delta)?)
                    }
                };
                Some(UiInputEvent::Wheel {
                    delta,
                    position: self.transform_optional_position(*position),
                })
            }
            event => Some(event.clone()),
        }
    }

    pub(crate) fn ordinary_event_allowed(&self, position: Option<Point>) -> bool {
        match position {
            Some(position) => self.accepts_screen_point(position),
            None => self.state.screen_to_local.is_some() && self.state.clips.is_empty(),
        }
    }

    pub(crate) fn transform_screen_point(&self, position: Point) -> Option<Point> {
        self.transform_event_position(position)
    }

    fn localize_ordinary_position(&self, position: Point) -> Option<Point> {
        if !self.accepts_screen_point(position) {
            return None;
        }
        self.transform_event_position(position)
    }

    fn transform_optional_position(&self, position: Option<Point>) -> Option<Point> {
        position.and_then(|position| self.transform_event_position(position))
    }

    fn transform_event_position(&self, position: Point) -> Option<Point> {
        let inverse = self.state.screen_to_local?;
        let position = inverse.transform_point(position);
        point_is_finite(position).then_some(position)
    }

    fn transform_event_vector(&self, vector: Vec2) -> Option<Vec2> {
        self.state
            .screen_to_local
            .and_then(|inverse| transform_vector(inverse, vector))
    }

    pub(crate) fn project_rect(&self, rect: Rect) -> Option<Rect> {
        self.state.screen_to_local?;
        let mut polygon = transformed_rect_polygon(rect, self.state.local_to_screen)?;
        if let Some(clip) = &self.state.effective_clip {
            polygon = intersect_convex(&polygon, clip);
        }
        polygon_bounds(&polygon)
    }

    pub(crate) fn project_semantic_rect(&self, rect: Rect) -> Option<Rect> {
        if !rect_is_finite(rect) || rect.width < 0.0 || rect.height < 0.0 {
            return None;
        }
        if rect.width == 0.0 && rect.height == 0.0 {
            self.state.screen_to_local?;
            let point = self.state.local_to_screen.transform_point(rect.min());
            return self
                .accepts_screen_point(point)
                .then_some(Rect::new(point.x, point.y, 0.0, 0.0));
        }
        self.project_rect(rect)
    }

    pub(crate) fn effective_clip_bounds(&self) -> Option<Rect> {
        self.state
            .effective_clip
            .as_deref()
            .and_then(polygon_bounds)
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.state.screen_to_local.is_some()
            && self
                .state
                .effective_clip
                .as_ref()
                .is_none_or(|clip| !clip.is_empty())
    }

    pub(crate) fn hit_test_rect(&self, screen_point: Option<Point>, rect: Rect) -> bool {
        if !rect_is_finite(rect) || rect.is_empty() {
            return false;
        }
        screen_point.is_some_and(|point| {
            self.accepts_screen_point(point)
                && self.state.screen_to_local.is_some_and(|inverse| {
                    let local = inverse.transform_point(point);
                    point_is_finite(local) && rect.contains_point(local)
                })
        })
    }

    fn accepts_screen_point(&self, point: Point) -> bool {
        point_is_finite(point)
            && self.state.screen_to_local.is_some()
            && self.state.clips.iter().all(|clip| clip.contains(point))
    }

    pub(crate) fn push_transform(&mut self, transform: Transform) {
        let previous = self.state.clone();
        self.scopes.push(SpatialScope::Transform(previous));
        self.state.local_to_screen = Transform::compose(self.state.local_to_screen, transform);
        self.state.screen_to_local = self.state.local_to_screen.try_inverse();
    }

    pub(crate) fn pop_transform(&mut self) {
        if matches!(self.scopes.last(), Some(SpatialScope::Transform(_))) {
            let Some(SpatialScope::Transform(previous)) = self.scopes.pop() else {
                return;
            };
            self.state = previous;
        }
    }

    pub(crate) fn push_clip(&mut self, id: ClipId, rect: Rect) {
        let previous = self.state.clone();
        self.scopes.push(SpatialScope::Clip { id, previous });

        let clip_polygon = transformed_rect_polygon(rect, self.state.local_to_screen)
            .filter(|_| self.state.screen_to_local.is_some())
            .unwrap_or_default();
        self.state.effective_clip = Some(match &self.state.effective_clip {
            Some(parent) => intersect_convex(parent, &clip_polygon),
            None => clip_polygon,
        });
        self.state.clips.push(TransformedClip {
            rect,
            screen_to_local: self.state.screen_to_local,
        });
    }

    pub(crate) fn pop_clip(&mut self, id: ClipId) {
        if !matches!(self.scopes.last(), Some(SpatialScope::Clip { id: open, .. }) if *open == id) {
            return;
        }
        let Some(SpatialScope::Clip { previous, .. }) = self.scopes.pop() else {
            return;
        };
        self.state = previous;
    }

    fn pop_layer(&mut self, id: LayerId) {
        if matches!(self.scopes.last(), Some(SpatialScope::Layer(open)) if *open == id) {
            self.scopes.pop();
        }
    }
}

fn release_cleanup(state: PointerButtonState, preserve: bool) -> PointerButtonState {
    PointerButtonState::new(false, false, preserve && state.released)
}

fn add_vectors(left: Vec2, right: Vec2) -> Vec2 {
    Vec2::new(left.x + right.x, left.y + right.y)
}

fn restore_release_edges(
    pointer: &mut crate::PointerInput,
    snapshot: &crate::PointerInput,
    preserve_primary: bool,
    preserve_secondary: bool,
) {
    if preserve_primary {
        pointer.primary.released |= snapshot.primary.released;
    }
    if preserve_secondary {
        pointer.secondary.released |= snapshot.secondary.released;
    }
}

#[derive(Debug, Clone)]
struct SpatialState {
    local_to_screen: Transform,
    screen_to_local: Option<Transform>,
    clips: Vec<TransformedClip>,
    effective_clip: Option<Vec<Point>>,
}

impl Default for SpatialState {
    fn default() -> Self {
        Self {
            local_to_screen: Transform::IDENTITY,
            screen_to_local: Some(Transform::IDENTITY),
            clips: Vec::new(),
            effective_clip: None,
        }
    }
}

#[derive(Debug, Clone)]
enum SpatialScope {
    Clip { id: ClipId, previous: SpatialState },
    Layer(LayerId),
    Transform(SpatialState),
}

#[derive(Debug, Clone, Copy)]
struct TransformedClip {
    rect: Rect,
    screen_to_local: Option<Transform>,
}

impl TransformedClip {
    fn contains(self, point: Point) -> bool {
        self.screen_to_local.is_some_and(|inverse| {
            let local = inverse.transform_point(point);
            point_is_finite(local) && rect_is_finite(self.rect) && self.rect.contains_point(local)
        })
    }
}

fn transformed_rect_polygon(rect: Rect, transform: Transform) -> Option<Vec<Point>> {
    if !rect_is_finite(rect) || rect.is_empty() || !transform.is_finite() {
        return None;
    }
    let mut polygon = vec![
        transform.transform_point(rect.min()),
        transform.transform_point(Point::new(rect.max_x(), rect.min_y())),
        transform.transform_point(rect.max()),
        transform.transform_point(Point::new(rect.min_x(), rect.max_y())),
    ];
    if polygon.iter().any(|point| !point_is_finite(*point)) {
        return None;
    }
    normalize_counter_clockwise(&mut polygon)?;
    Some(polygon)
}

fn intersect_convex(subject: &[Point], clip: &[Point]) -> Vec<Point> {
    if subject.len() < 3 || clip.len() < 3 {
        return Vec::new();
    }
    let mut output = subject.to_vec();
    for index in 0..clip.len() {
        let edge_start = clip[index];
        let edge_end = clip[(index + 1) % clip.len()];
        let input = std::mem::take(&mut output);
        if input.is_empty() {
            break;
        }
        let mut previous = *input.last().expect("non-empty polygon");
        let mut previous_inside = inside_edge(previous, edge_start, edge_end);
        for current in input {
            let current_inside = inside_edge(current, edge_start, edge_end);
            if current_inside != previous_inside
                && let Some(intersection) =
                    line_intersection(previous, current, edge_start, edge_end)
            {
                output.push(intersection);
            }
            if current_inside {
                output.push(current);
            }
            previous = current;
            previous_inside = current_inside;
        }
    }
    deduplicate_polygon(&mut output);
    output
}

fn inside_edge(point: Point, edge_start: Point, edge_end: Point) -> bool {
    cross(
        Vec2::new(edge_end.x - edge_start.x, edge_end.y - edge_start.y),
        Vec2::new(point.x - edge_start.x, point.y - edge_start.y),
    ) >= 0.0
}

fn line_intersection(
    start: Point,
    end: Point,
    edge_start: Point,
    edge_end: Point,
) -> Option<Point> {
    let edge = Vec2::new(edge_end.x - edge_start.x, edge_end.y - edge_start.y);
    let segment = Vec2::new(end.x - start.x, end.y - start.y);
    let denominator = cross(edge, segment);
    if !denominator.is_finite() || denominator == 0.0 {
        return None;
    }
    let from_edge = Vec2::new(start.x - edge_start.x, start.y - edge_start.y);
    let amount = (-cross(edge, from_edge) / denominator).clamp(0.0, 1.0);
    let point = Point::new(
        segment.x.mul_add(amount, start.x),
        segment.y.mul_add(amount, start.y),
    );
    point_is_finite(point).then_some(point)
}

fn polygon_bounds(polygon: &[Point]) -> Option<Rect> {
    let first = *polygon.first()?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
    for point in &polygon[1..] {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    let bounds = Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y));
    (!bounds.is_empty() && rect_is_finite(bounds)).then_some(bounds)
}

fn normalize_counter_clockwise(polygon: &mut [Point]) -> Option<()> {
    let area = signed_area(polygon);
    if !area.is_finite() || area == 0.0 {
        return None;
    }
    if area < 0.0 {
        polygon.reverse();
    }
    Some(())
}

fn signed_area(polygon: &[Point]) -> f32 {
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
        .map(|(current, next)| current.x.mul_add(next.y, -(next.x * current.y)))
        .sum::<f32>()
        * 0.5
}

fn deduplicate_polygon(polygon: &mut Vec<Point>) {
    polygon.dedup_by(|left, right| left == right);
    if polygon.len() > 1 {
        let first = polygon[0];
        let last = *polygon.last().expect("polygon has at least two points");
        if first == last {
            polygon.pop();
        }
    }
}

fn transform_vector(transform: Transform, vector: Vec2) -> Option<Vec2> {
    let transformed = Vec2::new(
        transform.m11.mul_add(vector.x, transform.m21 * vector.y),
        transform.m12.mul_add(vector.x, transform.m22 * vector.y),
    );
    (transformed.x.is_finite() && transformed.y.is_finite()).then_some(transformed)
}

fn cross(left: Vec2, right: Vec2) -> f32 {
    left.x.mul_add(right.y, -(left.y * right.x))
}

fn point_is_finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn rect_is_finite(rect: Rect) -> bool {
    rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite()
}
