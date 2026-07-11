//! Frame-local pointer target arbitration.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use crate::memory::PlannedDragRelease;
use crate::{
    ClipId, MouseButton, Point, PointerRoute, PointerRoutes, Rect, Transform, UiInputEvent,
    WidgetId,
};

use super::spatial::SpatialStack;
use crate::interaction::crosses_drag_threshold;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PointerDropProbe {
    Snapshot,
    Release {
        ordinal: usize,
        position: Option<Point>,
    },
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PointerPressProbe {
    pub ordinal: usize,
    pub position: Option<Point>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RetainedDragProbe {
    pub source: WidgetId,
    pub origin: Option<Point>,
    pub threshold_crossed: bool,
}

/// Explicit back-to-front paint ordinal used by pointer arbitration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PointerOrder(u64);

impl PointerOrder {
    /// Creates an explicit paint ordinal.
    #[must_use]
    pub const fn new(ordinal: u64) -> Self {
        Self(ordinal)
    }

    /// Returns the raw paint ordinal.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// One visual region participating in ordinary and drop routing.
#[derive(Debug, Clone, PartialEq)]
pub struct PointerTarget {
    canonical: WidgetId,
    rect: Rect,
    order: PointerOrder,
    ordinary_owner: Option<WidgetId>,
    drop_owner: Option<WidgetId>,
    wheel_owner: Option<WidgetId>,
    cursor_equivalents: Vec<WidgetId>,
    domain_drag_source: bool,
    enabled: bool,
}

impl PointerTarget {
    /// Creates an enabled visual target whose canonical ID owns ordinary events.
    #[must_use]
    pub fn new(canonical: WidgetId, rect: Rect, order: PointerOrder) -> Self {
        Self {
            canonical,
            rect,
            order,
            ordinary_owner: Some(canonical),
            drop_owner: None,
            wheel_owner: None,
            cursor_equivalents: Vec::new(),
            domain_drag_source: false,
            enabled: true,
        }
    }

    /// Replaces the exact ordinary event owner.
    #[must_use]
    pub const fn ordinary_owner(mut self, owner: Option<WidgetId>) -> Self {
        self.ordinary_owner = owner;
        self
    }

    /// Assigns the exact drop destination owner for this visual target.
    #[must_use]
    pub const fn drop_owner(mut self, owner: WidgetId) -> Self {
        self.drop_owner = Some(owner);
        self
    }

    /// Assigns the exact scroll viewport allowed to consume wheel input.
    #[must_use]
    pub const fn wheel_owner(mut self, owner: WidgetId) -> Self {
        self.wheel_owner = Some(owner);
        self
    }

    /// Declares that this target's ordinary owner resolves a `DomainDrag` gesture.
    ///
    /// Closed pointer plans use this intent to derive causal same-frame and
    /// target-first drop commits without speculating that every pressable is a
    /// drag source.
    #[must_use]
    pub const fn domain_drag_source(mut self) -> Self {
        self.domain_drag_source = true;
        self
    }

    /// Creates an enabled wheel-only visual region.
    #[must_use]
    pub fn wheel_only(owner: WidgetId, rect: Rect, order: PointerOrder) -> Self {
        Self::new(owner, rect, order)
            .ordinary_owner(None)
            .wheel_owner(owner)
    }

    /// Adds an ID that may publish cursor intent for the same captured target.
    ///
    /// Cursor equivalence does not authorize hover, press, click, or capture.
    #[must_use]
    pub fn cursor_equivalent(mut self, id: WidgetId) -> Self {
        self.cursor_equivalents.push(id);
        self
    }

    /// Marks this target eligible or ineligible.
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Deterministic pointer-plan validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerPlanError {
    /// A frame attempted to install more than one closed target plan.
    AlreadyInstalled,
    /// Two declarations used the same explicit paint order.
    DuplicateOrder(PointerOrder),
    /// One widget ID was assigned to different descriptors.
    ConflictingWidgetId(WidgetId),
}

impl fmt::Display for PointerPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInstalled => formatter.write_str("pointer target plan already installed"),
            Self::DuplicateOrder(order) => {
                write!(formatter, "duplicate pointer paint order {}", order.raw())
            }
            Self::ConflictingWidgetId(id) => {
                write!(
                    formatter,
                    "pointer widget ID {id:?} belongs to multiple targets"
                )
            }
        }
    }
}

impl Error for PointerPlanError {}

/// Builder for one closed-world, frame-local pointer target plan.
///
/// Layout code declares target geometry and explicit paint order before the
/// first routed behavior call. Declarations may arrive in any order.
pub struct PointerTargetPlan {
    pointer: Option<Point>,
    press_probe: Option<PointerPressProbe>,
    drop_probe: PointerDropProbe,
    retained_drag: Option<RetainedDragProbe>,
    root_events: Vec<UiInputEvent>,
    spatial: SpatialStack,
    targets: Vec<ResolvedTarget>,
    blockers: Vec<ResolvedBlocker>,
    barriers: Vec<PointerOrder>,
    orders: HashSet<PointerOrder>,
    id_descriptors: HashMap<WidgetId, u64>,
    next_descriptor: u64,
    next_clip: u64,
    error: Option<PointerPlanError>,
}

impl PointerTargetPlan {
    pub(crate) fn new(
        pointer: Option<Point>,
        press_probe: Option<PointerPressProbe>,
        drop_probe: PointerDropProbe,
        retained_drag: Option<RetainedDragProbe>,
        root_events: Vec<UiInputEvent>,
        spatial: SpatialStack,
    ) -> Self {
        Self {
            pointer,
            press_probe,
            drop_probe,
            retained_drag,
            root_events,
            spatial,
            targets: Vec::new(),
            blockers: Vec::new(),
            barriers: Vec::new(),
            orders: HashSet::new(),
            id_descriptors: HashMap::new(),
            next_descriptor: 0,
            next_clip: u64::MAX,
            error: None,
        }
    }

    /// Adds one visual target in the current plan-local spatial scope.
    pub fn target(&mut self, target: PointerTarget) {
        let descriptor = self.next_descriptor();
        self.record_order(target.order);
        self.record_id(target.canonical, descriptor);
        if let Some(owner) = target.ordinary_owner {
            self.record_id(owner, descriptor);
        }
        if let Some(owner) = target.drop_owner {
            self.record_id(owner, descriptor);
        }
        if let Some(owner) = target.wheel_owner {
            self.record_id(owner, descriptor);
        }
        for equivalent in &target.cursor_equivalents {
            self.record_id(*equivalent, descriptor);
        }

        let valid = target.enabled && self.spatial.project_rect(target.rect).is_some();
        let ordinary_hit = valid
            && self
                .spatial
                .hit_test_rect(self.interaction_pointer(), target.rect);
        let wheel_hit = valid && self.spatial.hit_test_rect(self.pointer, target.rect);
        let drop_hit = valid
            && !matches!(self.drop_probe, PointerDropProbe::Cancelled)
            && self.spatial.hit_test_rect(self.drop_pointer(), target.rect);
        let drop_event_allowed = match self.drop_probe {
            PointerDropProbe::Snapshot => self.spatial.ordinary_event_allowed(self.pointer),
            PointerDropProbe::Release { position, .. } => {
                self.spatial.ordinary_event_allowed(position)
            }
            PointerDropProbe::Cancelled => false,
        };
        let mut cursor_equivalents = target.cursor_equivalents;
        cursor_equivalents.push(target.canonical);
        if let Some(owner) = target.ordinary_owner {
            cursor_equivalents.push(owner);
        }
        if let Some(owner) = target.drop_owner {
            cursor_equivalents.push(owner);
        }
        cursor_equivalents.sort_unstable();
        cursor_equivalents.dedup();
        self.targets.push(ResolvedTarget {
            order: target.order,
            ordinary_owner: target.ordinary_owner,
            drop_owner: target.drop_owner,
            wheel_owner: target.wheel_owner,
            cursor_equivalents,
            domain_drag_source: target.domain_drag_source,
            spatial: self.spatial.clone(),
            valid,
            ordinary_hit,
            wheel_hit,
            drop_hit,
            drop_event_allowed,
        });
    }

    /// Adds a non-interactive visual region that blocks all lower pointer routes.
    pub fn blocker(&mut self, rect: Rect, order: PointerOrder) {
        self.next_descriptor();
        self.record_order(order);
        let valid = self.spatial.project_rect(rect).is_some();
        let ordinary_hit = valid && self.spatial.hit_test_rect(self.interaction_pointer(), rect);
        let wheel_hit = valid && self.spatial.hit_test_rect(self.pointer, rect);
        let drop_hit = valid
            && !matches!(self.drop_probe, PointerDropProbe::Cancelled)
            && self.spatial.hit_test_rect(self.drop_pointer(), rect);
        self.blockers.push(ResolvedBlocker {
            order,
            valid,
            ordinary_hit,
            wheel_hit,
            drop_hit,
        });
    }

    /// Adds a barrier that blocks every declaration at a lower paint order.
    ///
    /// Modal layers and outside-click overlays use this even when the pointer is
    /// outside their painted rectangle.
    pub fn capture_lower_layers(&mut self, order: PointerOrder) {
        self.next_descriptor();
        self.record_order(order);
        self.barriers.push(order);
    }

    /// Runs declarations under an additional affine transform.
    pub fn with_transform<T>(
        &mut self,
        transform: Transform,
        declare: impl FnOnce(&mut Self) -> T,
    ) -> T {
        self.spatial.push_transform(transform);
        let output = declare(self);
        self.spatial.pop_transform();
        output
    }

    /// Runs declarations under an additional exact transformed clip.
    pub fn with_clip<T>(&mut self, rect: Rect, declare: impl FnOnce(&mut Self) -> T) -> T {
        let id = ClipId::from_raw(self.next_clip);
        self.next_clip = self.next_clip.saturating_sub(1);
        self.spatial.push_clip(id, rect);
        let output = declare(self);
        self.spatial.pop_clip(id);
        output
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn resolve(
        self,
        captured: Option<WidgetId>,
    ) -> Result<ResolvedPointerPlan, PointerPlanError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        let barrier = self.barriers.into_iter().max();
        let eligible = |order: PointerOrder| barrier.is_none_or(|floor| order > floor);

        let top_target = self
            .targets
            .iter()
            .filter(|target| target.valid && target.ordinary_hit && eligible(target.order))
            .max_by_key(|target| target.order);
        let top_blocker = self
            .blockers
            .iter()
            .filter(|blocker| blocker.valid && blocker.ordinary_hit && eligible(blocker.order))
            .max_by_key(|blocker| blocker.order);
        let top_visual = match (top_target, top_blocker) {
            (Some(target), Some(blocker)) if blocker.order > target.order => TopVisual::Blocker,
            (Some(target), _) => TopVisual::Target(target),
            (None, Some(_)) => TopVisual::Blocker,
            (None, None) => TopVisual::None,
        };

        let top_drop_target = self
            .targets
            .iter()
            .filter(|target| target.valid && target.drop_hit && eligible(target.order))
            .max_by_key(|target| target.order);
        let top_drop_blocker = self
            .blockers
            .iter()
            .filter(|blocker| blocker.valid && blocker.drop_hit && eligible(blocker.order))
            .max_by_key(|blocker| blocker.order);
        let top_drop_visual = match (top_drop_target, top_drop_blocker) {
            (Some(target), Some(blocker)) if blocker.order > target.order => TopVisual::Blocker,
            (Some(target), _) => TopVisual::Target(target),
            (None, Some(_)) => TopVisual::Blocker,
            (None, None) => TopVisual::None,
        };

        let captured_target = captured.and_then(|owner| {
            self.targets.iter().find(|target| {
                target.valid && eligible(target.order) && target.ordinary_owner == Some(owner)
            })
        });
        let ordinary_target = captured_target.or(match top_visual {
            TopVisual::Target(target) => Some(target),
            TopVisual::Blocker | TopVisual::None => None,
        });
        let ordinary = ordinary_target
            .and_then(|target| target.ordinary_owner)
            .map_or(PointerRoute::Blocked, PointerRoute::Target);
        let transaction_source_target = if captured.is_some() {
            captured_target
        } else if self.press_probe.is_some() {
            ordinary_target
        } else {
            None
        };
        let source_probe_requires_validation = matches!(
            self.drop_probe,
            PointerDropProbe::Release { .. } | PointerDropProbe::Snapshot
        ) && transaction_source_target.is_some();
        let drop_source_valid = !source_probe_requires_validation
            || transaction_source_target.is_some_and(|target| {
                target.domain_drag_source
                    && target.drop_event_allowed
                    && planned_source_matches(
                        target,
                        self.press_probe,
                        self.retained_drag,
                        captured,
                    )
            });
        let drop = if drop_source_valid {
            match top_drop_visual {
                TopVisual::Target(target) => target
                    .drop_owner
                    .map_or(PointerRoute::Blocked, PointerRoute::Target),
                TopVisual::Blocker | TopVisual::None => PointerRoute::Blocked,
            }
        } else {
            PointerRoute::Blocked
        };
        let planned_drag_release = match drop {
            PointerRoute::Target(_) => transaction_source_target.and_then(|target| {
                planned_drag_release(
                    target,
                    self.press_probe,
                    self.retained_drag,
                    self.drop_probe,
                    &self.root_events,
                )
            }),
            PointerRoute::Unplanned | PointerRoute::Blocked => None,
        };
        let planned_drag_source = match drop {
            PointerRoute::Target(_) => transaction_source_target.and_then(|target| {
                planned_active_drag_source(
                    target,
                    self.press_probe,
                    self.retained_drag,
                    self.drop_probe,
                    &self.root_events,
                )
            }),
            PointerRoute::Unplanned | PointerRoute::Blocked => None,
        };

        let top_wheel = self
            .targets
            .iter()
            .filter(|target| {
                target.valid
                    && target.wheel_hit
                    && target.wheel_owner.is_some()
                    && eligible(target.order)
            })
            .max_by_key(|target| target.order);
        let top_wheel_blocker = self
            .blockers
            .iter()
            .filter(|blocker| blocker.valid && blocker.wheel_hit && eligible(blocker.order))
            .max_by_key(|blocker| blocker.order);
        let wheel = match (top_wheel, top_wheel_blocker) {
            (Some(target), Some(blocker)) if blocker.order > target.order => PointerRoute::Blocked,
            (Some(target), _) => PointerRoute::Target(
                target
                    .wheel_owner
                    .expect("filtered pointer target has wheel owner"),
            ),
            (None, _) => PointerRoute::Blocked,
        };

        Ok(ResolvedPointerPlan {
            routes: PointerRoutes {
                ordinary,
                drop,
                wheel,
            },
            cursor_equivalents: ordinary_target
                .map(|target| target.cursor_equivalents.clone())
                .unwrap_or_default(),
            capture_valid: captured.is_none() || captured_target.is_some(),
            planned_drag_release,
            planned_drag_source,
        })
    }

    fn next_descriptor(&mut self) -> u64 {
        let descriptor = self.next_descriptor;
        self.next_descriptor = self.next_descriptor.saturating_add(1);
        descriptor
    }

    fn drop_pointer(&self) -> Option<Point> {
        match self.drop_probe {
            PointerDropProbe::Snapshot => self.pointer,
            PointerDropProbe::Release { position, .. } => position,
            PointerDropProbe::Cancelled => None,
        }
    }

    fn interaction_pointer(&self) -> Option<Point> {
        self.press_probe
            .map_or(self.pointer, |probe| probe.position)
    }

    fn record_order(&mut self, order: PointerOrder) {
        if !self.orders.insert(order) && self.error.is_none() {
            self.error = Some(PointerPlanError::DuplicateOrder(order));
        }
    }

    fn record_id(&mut self, id: WidgetId, descriptor: u64) {
        if self
            .id_descriptors
            .insert(id, descriptor)
            .is_some_and(|previous| previous != descriptor)
            && self.error.is_none()
        {
            self.error = Some(PointerPlanError::ConflictingWidgetId(id));
        }
    }
}

fn planned_source_matches(
    target: &ResolvedTarget,
    press_probe: Option<PointerPressProbe>,
    retained_drag: Option<RetainedDragProbe>,
    captured: Option<WidgetId>,
) -> bool {
    if let Some(retained) = retained_drag {
        return captured == Some(retained.source) && target.ordinary_owner == Some(retained.source);
    }
    captured.is_none() && press_probe.is_some() && target.ordinary_owner.is_some()
}

fn planned_drag_release(
    target: &ResolvedTarget,
    press_probe: Option<PointerPressProbe>,
    retained_drag: Option<RetainedDragProbe>,
    drop_probe: PointerDropProbe,
    root_events: &[UiInputEvent],
) -> Option<PlannedDragRelease> {
    if !target.domain_drag_source || !target.drop_event_allowed {
        return None;
    }
    let PointerDropProbe::Release {
        ordinal: release_ordinal,
        ..
    } = drop_probe
    else {
        return None;
    };

    let (source, origin, mut threshold_crossed, first_motion_ordinal) =
        if let Some(retained) = retained_drag {
            (
                retained.source,
                retained.origin,
                retained.threshold_crossed,
                0,
            )
        } else {
            let press = press_probe?;
            let source = target.ordinary_owner?;
            let origin = press
                .position
                .and_then(|position| target.spatial.transform_screen_point(position));
            (source, origin, false, press.ordinal.saturating_add(1))
        };
    if target.ordinary_owner != Some(source) {
        return None;
    }

    for (ordinal, event) in root_events.iter().enumerate() {
        if ordinal < first_motion_ordinal {
            continue;
        }
        if ordinal > release_ordinal {
            break;
        }
        let position = match event {
            UiInputEvent::PointerMoved { position, .. } => Some(*position),
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: false,
                position,
                ..
            } if ordinal == release_ordinal => *position,
            _ => None,
        };
        if let (Some(origin), Some(position)) = (origin, position)
            && target.spatial.ordinary_event_allowed(Some(position))
            && let Some(position) = target.spatial.transform_screen_point(position)
        {
            threshold_crossed |= crosses_drag_threshold(origin, position);
        }
        if ordinal == release_ordinal {
            return threshold_crossed.then_some(PlannedDragRelease {
                source,
                ordinal: release_ordinal,
            });
        }
    }
    None
}

fn planned_active_drag_source(
    target: &ResolvedTarget,
    press_probe: Option<PointerPressProbe>,
    retained_drag: Option<RetainedDragProbe>,
    drop_probe: PointerDropProbe,
    root_events: &[UiInputEvent],
) -> Option<WidgetId> {
    if !target.domain_drag_source
        || !target.drop_event_allowed
        || drop_probe != PointerDropProbe::Snapshot
    {
        return None;
    }

    let (source, origin, mut threshold_crossed, first_motion_ordinal) =
        if let Some(retained) = retained_drag {
            (
                retained.source,
                retained.origin,
                retained.threshold_crossed,
                0,
            )
        } else {
            let press = press_probe?;
            let source = target.ordinary_owner?;
            let origin = press
                .position
                .and_then(|position| target.spatial.transform_screen_point(position));
            (source, origin, false, press.ordinal.saturating_add(1))
        };
    if target.ordinary_owner != Some(source) {
        return None;
    }

    for (ordinal, event) in root_events.iter().enumerate() {
        if ordinal < first_motion_ordinal {
            continue;
        }
        match event {
            UiInputEvent::PointerMoved { position, .. } => {
                if let Some(origin) = origin
                    && target.spatial.ordinary_event_allowed(Some(*position))
                    && let Some(position) = target.spatial.transform_screen_point(*position)
                {
                    threshold_crossed |= crosses_drag_threshold(origin, position);
                }
            }
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: false,
                ..
            }
            | UiInputEvent::PointerReleaseAll { .. }
            | UiInputEvent::WindowFocusChanged(false) => return None,
            _ => {}
        }
    }
    threshold_crossed.then_some(source)
}

pub(crate) struct ResolvedPointerPlan {
    pub(crate) routes: PointerRoutes,
    pub(crate) cursor_equivalents: Vec<WidgetId>,
    pub(crate) capture_valid: bool,
    pub(crate) planned_drag_release: Option<PlannedDragRelease>,
    pub(crate) planned_drag_source: Option<WidgetId>,
}

#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
struct ResolvedTarget {
    order: PointerOrder,
    ordinary_owner: Option<WidgetId>,
    drop_owner: Option<WidgetId>,
    wheel_owner: Option<WidgetId>,
    cursor_equivalents: Vec<WidgetId>,
    domain_drag_source: bool,
    spatial: SpatialStack,
    valid: bool,
    ordinary_hit: bool,
    wheel_hit: bool,
    drop_hit: bool,
    drop_event_allowed: bool,
}

#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
struct ResolvedBlocker {
    order: PointerOrder,
    valid: bool,
    ordinary_hit: bool,
    wheel_hit: bool,
    drop_hit: bool,
}

#[derive(Debug, Clone, Copy)]
enum TopVisual<'a> {
    Target(&'a ResolvedTarget),
    Blocker,
    None,
}
