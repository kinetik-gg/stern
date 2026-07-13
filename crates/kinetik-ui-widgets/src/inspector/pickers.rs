//! Retained inspector picker-flow state and host-service boundary.

use std::collections::{BTreeMap, BTreeSet};

use kinetik_ui_core::{Color, PointerOrder, PointerTarget, PointerTargetPlan, Rect, WidgetId};

use crate::components::{AssetSlotOutput, ColorFieldOutput, PathFieldOutput, SelectFieldOutput};
use crate::overlays::{
    DropdownItem, DropdownItemId, DropdownModel, DropdownOverlay, OverlayDismissal, OverlayEntry,
    OverlayId, OverlayKind, OverlayScene, OverlaySceneSurface,
};

const MIN_COLOR_PICKER_WIDTH: f32 = 128.0;
const MIN_COLOR_PICKER_HEIGHT: f32 = 148.0;

/// Inspector picker kind currently owned by [`InspectorPickerState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorPickerKind {
    /// Select/enum dropdown.
    Select,
    /// RGBA color editor.
    Color,
    /// Application asset chooser.
    Asset,
    /// Host-provided path chooser.
    Path,
}

/// One application-owned asset choice shown in an inspector picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetPickerItem {
    /// Stable UI identity used for highlight and selection.
    pub id: DropdownItemId,
    /// Stable application asset identity returned on commit.
    pub identity: String,
    /// User-visible label.
    pub label: String,
    /// Whether this item can be selected.
    pub enabled: bool,
}

impl AssetPickerItem {
    /// Creates an enabled asset choice.
    #[must_use]
    pub fn new(id: DropdownItemId, identity: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id,
            identity: identity.into(),
            label: label.into(),
            enabled: true,
        }
    }

    /// Sets whether this choice may be selected.
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Application-owned value committed by a completed picker flow.
#[derive(Debug, Clone, PartialEq)]
pub enum InspectorPickerCommit {
    /// Selected dropdown item.
    Select(DropdownItemId),
    /// Applied normalized RGBA color.
    Color(Color),
    /// Selected stable asset identity.
    Asset(String),
    /// Selected path returned by the host service.
    Path(String),
}

/// Reason an inspector picker closed without committing a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorPickerCancelReason {
    /// Escape closed an overlay.
    Escape,
    /// Pointer activation outside the overlay closed it.
    OutsideClick,
    /// An explicit Cancel control closed it.
    Explicit,
    /// The host path chooser was cancelled.
    ServiceCancelled,
    /// The host path chooser failed or returned an invalid value.
    ServiceFailed,
}

/// Host chooser type requested for a path field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPickerKind {
    /// Select one file.
    File,
    /// Select one directory.
    Directory,
}

/// Redacted, one-shot request for an application/platform path service.
///
/// The request deliberately contains no current path or filesystem contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathPickerRequest {
    /// Monotonic session generation used to reject stale results.
    pub generation: u64,
    /// Exact browse trigger that owns the request and focus return.
    pub trigger: WidgetId,
    /// Host chooser type.
    pub kind: PathPickerKind,
}

/// Host outcome for a matching [`PathPickerRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathPickerOutcome {
    /// Host selected one path.
    Selected(String),
    /// User cancelled the host chooser.
    Cancelled,
    /// Host could not complete the chooser.
    Failed,
}

/// Generation- and target-checked result returned by a host path service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPickerResult {
    /// Request generation being resolved.
    pub generation: u64,
    /// Trigger that originally owned the request.
    pub trigger: WidgetId,
    /// Host result.
    pub outcome: PathPickerOutcome,
}

impl PathPickerResult {
    /// Creates a path service result.
    #[must_use]
    pub const fn new(generation: u64, trigger: WidgetId, outcome: PathPickerOutcome) -> Self {
        Self {
            generation,
            trigger,
            outcome,
        }
    }

    /// Creates a selected-path result for a request.
    #[must_use]
    pub fn selected(request: PathPickerRequest, path: impl Into<String>) -> Self {
        Self::new(
            request.generation,
            request.trigger,
            PathPickerOutcome::Selected(path.into()),
        )
    }

    /// Creates a cancelled result for a request.
    #[must_use]
    pub const fn cancelled(request: PathPickerRequest) -> Self {
        Self::new(
            request.generation,
            request.trigger,
            PathPickerOutcome::Cancelled,
        )
    }

    /// Creates a failed result for a request.
    #[must_use]
    pub const fn failed(request: PathPickerRequest) -> Self {
        Self::new(
            request.generation,
            request.trigger,
            PathPickerOutcome::Failed,
        )
    }
}

/// Output from evaluating or resolving one inspector picker frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InspectorPickerOutput {
    /// Picker kind active at the start of evaluation.
    pub active: Option<InspectorPickerKind>,
    /// Application-owned value committed exactly once.
    pub commit: Option<InspectorPickerCommit>,
    /// Close reason when no value was committed.
    pub cancel: Option<InspectorPickerCancelReason>,
    /// One-shot redacted path-service request.
    pub service_request: Option<PathPickerRequest>,
    /// Trigger that regained or should regain focus after resolution.
    pub focus_return: Option<WidgetId>,
}

/// Retained single-session coordinator for inspector picker flows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InspectorPickerState {
    next_generation: u64,
    session: Option<InspectorPickerSession>,
}

impl InspectorPickerState {
    /// Creates an idle picker coordinator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_generation: 0,
            session: None,
        }
    }

    /// Returns the active picker kind.
    #[must_use]
    pub const fn kind(&self) -> Option<InspectorPickerKind> {
        match &self.session {
            Some(InspectorPickerSession::Scene(scene)) => Some(scene.kind()),
            Some(InspectorPickerSession::Path(_)) => Some(InspectorPickerKind::Path),
            None => None,
        }
    }

    /// Returns the prepared overlay scene used for pointer planning.
    #[must_use]
    pub const fn scene(&self) -> Option<&InspectorPickerScene> {
        match &self.session {
            Some(InspectorPickerSession::Scene(scene)) => Some(scene),
            Some(InspectorPickerSession::Path(_)) | None => None,
        }
    }

    /// Returns the current color draft without changing application state.
    #[must_use]
    pub const fn color_draft(&self) -> Option<Color> {
        match &self.session {
            Some(InspectorPickerSession::Scene(InspectorPickerScene {
                kind: InspectorPickerSceneKind::Color(scene),
                ..
            })) => Some(scene.draft),
            _ => None,
        }
    }

    /// Opens a select overlay from a live entry-field request.
    pub fn open_select_from(
        &mut self,
        field: &SelectFieldOutput,
        overlay_id: OverlayId,
        bounds: Rect,
        label: impl Into<String>,
        model: &DropdownModel,
    ) -> bool {
        if self.session.is_some()
            || !field.open_requested
            || field.read_only
            || field.response.state.disabled
            || !valid_overlay_bounds(bounds)
            || !valid_dropdown_items(model.items())
        {
            return false;
        }
        let mut model = model.clone();
        ensure_highlight(&mut model);
        let scene = dropdown_scene(
            InspectorPickerKind::Select,
            field.response.id,
            overlay_id,
            bounds,
            label.into(),
            model,
            BTreeMap::new(),
        );
        self.session = Some(InspectorPickerSession::Scene(scene));
        true
    }

    /// Reconciles a live select model while retaining valid identity state.
    pub fn reconcile_select(&mut self, model: &DropdownModel) -> bool {
        let Some(scene) = self.scene_mut() else {
            return false;
        };
        let InspectorPickerSceneKind::Select { scene, .. } = &mut scene.kind else {
            return false;
        };
        if !unique_dropdown_items(model.items()) {
            return false;
        }
        let Some(OverlaySceneSurface::Dropdown { overlay, .. }) = scene.surfaces_mut().first_mut()
        else {
            return false;
        };
        overlay.model.replace_items(model.items().iter().cloned());
        overlay.model.clear_selection();
        if let Some(selected) = model.selected_id() {
            let _ = overlay.model.set_selected_id(selected);
        }
        ensure_highlight(&mut overlay.model);
        true
    }

    /// Opens a color overlay from a live entry-field request.
    ///
    /// Bounds smaller than 128 by 148 logical units are rejected so the four
    /// component rows and Apply/Cancel controls never overlap.
    pub fn open_color_from(
        &mut self,
        field: &ColorFieldOutput,
        overlay_id: OverlayId,
        bounds: Rect,
    ) -> bool {
        if self.session.is_some()
            || !field.open_requested
            || field.read_only
            || field.response.state.disabled
            || !valid_color_overlay_bounds(bounds)
        {
            return false;
        }
        self.session = Some(InspectorPickerSession::Scene(InspectorPickerScene {
            kind: InspectorPickerSceneKind::Color(ColorPickerScene::new(
                field.response.id,
                overlay_id,
                bounds,
                field.color,
            )),
            opened_frame: None,
        }));
        true
    }

    /// Opens an asset overlay from a live entry-field request.
    pub fn open_asset_from(
        &mut self,
        field: &AssetSlotOutput,
        overlay_id: OverlayId,
        bounds: Rect,
        label: impl Into<String>,
        items: &[AssetPickerItem],
    ) -> bool {
        if self.session.is_some()
            || !field.pick_requested
            || field.read_only
            || field.response.state.disabled
            || !valid_overlay_bounds(bounds)
            || !valid_asset_items(items)
        {
            return false;
        }
        let mut model =
            DropdownModel::from_items(items.iter().map(|item| {
                DropdownItem::new(item.id, item.label.clone()).with_enabled(item.enabled)
            }));
        ensure_highlight(&mut model);
        let identities = items
            .iter()
            .map(|item| (item.id, item.identity.clone()))
            .collect();
        let scene = dropdown_scene(
            InspectorPickerKind::Asset,
            field.response.id,
            overlay_id,
            bounds,
            label.into(),
            model,
            identities,
        );
        self.session = Some(InspectorPickerSession::Scene(scene));
        true
    }

    /// Reconciles asset choices while retaining valid identity state.
    pub fn reconcile_assets(&mut self, items: &[AssetPickerItem]) -> bool {
        if !unique_asset_items(items) {
            return false;
        }
        let Some(scene) = self.scene_mut() else {
            return false;
        };
        let InspectorPickerSceneKind::Asset {
            scene, identities, ..
        } = &mut scene.kind
        else {
            return false;
        };
        let Some(OverlaySceneSurface::Dropdown { overlay, .. }) = scene.surfaces_mut().first_mut()
        else {
            return false;
        };
        overlay.model.replace_items(
            items.iter().map(|item| {
                DropdownItem::new(item.id, item.label.clone()).with_enabled(item.enabled)
            }),
        );
        ensure_highlight(&mut overlay.model);
        *identities = items
            .iter()
            .map(|item| (item.id, item.identity.clone()))
            .collect();
        true
    }

    /// Opens a one-shot host path chooser from a live browse request.
    pub fn open_path_from(&mut self, field: &PathFieldOutput, kind: PathPickerKind) -> bool {
        let Some(trigger) = field.browse_response.as_ref().map(|response| response.id) else {
            return false;
        };
        if self.session.is_some()
            || !field.browse_requested
            || field.read_only
            || field
                .browse_response
                .as_ref()
                .is_some_and(|response| response.state.disabled)
        {
            return false;
        }
        self.next_generation = self.next_generation.wrapping_add(1);
        if self.next_generation == 0 {
            self.next_generation = 1;
        }
        let request = PathPickerRequest {
            generation: self.next_generation,
            trigger,
            kind,
        };
        self.session = Some(InspectorPickerSession::Path(PathPickerSession {
            request,
            emitted: false,
        }));
        true
    }

    /// Takes the pending path request once while retaining its active session.
    pub fn take_path_service_request(&mut self) -> Option<PathPickerRequest> {
        let Some(InspectorPickerSession::Path(session)) = self.session.as_mut() else {
            return None;
        };
        if session.emitted {
            return None;
        }
        session.emitted = true;
        Some(session.request)
    }

    /// Resolves a matching host path result once; stale results are ignored.
    pub fn resolve_path_result(
        &mut self,
        result: PathPickerResult,
    ) -> Option<InspectorPickerOutput> {
        let Some(InspectorPickerSession::Path(session)) = self.session.as_ref() else {
            return None;
        };
        if result.generation != session.request.generation
            || result.trigger != session.request.trigger
            || !session.emitted
        {
            return None;
        }
        let trigger = session.request.trigger;
        self.session = None;
        let mut output = InspectorPickerOutput {
            active: Some(InspectorPickerKind::Path),
            focus_return: Some(trigger),
            ..InspectorPickerOutput::default()
        };
        match result.outcome {
            PathPickerOutcome::Selected(path) if !path.is_empty() => {
                output.commit = Some(InspectorPickerCommit::Path(path));
            }
            PathPickerOutcome::Selected(_) | PathPickerOutcome::Failed => {
                output.cancel = Some(InspectorPickerCancelReason::ServiceFailed);
            }
            PathPickerOutcome::Cancelled => {
                output.cancel = Some(InspectorPickerCancelReason::ServiceCancelled);
            }
        }
        Some(output)
    }

    pub(crate) fn take_scene(&mut self) -> Option<InspectorPickerScene> {
        match self.session.take() {
            Some(InspectorPickerSession::Scene(scene)) => Some(scene),
            Some(path @ InspectorPickerSession::Path(_)) => {
                self.session = Some(path);
                None
            }
            None => None,
        }
    }

    pub(crate) fn restore_scene(&mut self, scene: InspectorPickerScene) {
        debug_assert!(self.session.is_none());
        self.session = Some(InspectorPickerSession::Scene(scene));
    }

    pub(crate) fn mark_scene_opened_frame(&mut self, frame_index: u64) {
        if let Some(scene) = self.scene_mut() {
            scene.opened_frame = Some(frame_index);
        }
    }

    fn scene_mut(&mut self) -> Option<&mut InspectorPickerScene> {
        match &mut self.session {
            Some(InspectorPickerSession::Scene(scene)) => Some(scene),
            Some(InspectorPickerSession::Path(_)) | None => None,
        }
    }
}

/// Prepared real overlay scene for one select, color, or asset picker.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectorPickerScene {
    pub(crate) kind: InspectorPickerSceneKind,
    pub(crate) opened_frame: Option<u64>,
}

impl InspectorPickerScene {
    /// Returns the prepared picker kind.
    #[must_use]
    pub const fn kind(&self) -> InspectorPickerKind {
        match self.kind {
            InspectorPickerSceneKind::Select { .. } => InspectorPickerKind::Select,
            InspectorPickerSceneKind::Color(_) => InspectorPickerKind::Color,
            InspectorPickerSceneKind::Asset { .. } => InspectorPickerKind::Asset,
        }
    }

    /// Returns the originating trigger.
    #[must_use]
    pub const fn trigger(&self) -> WidgetId {
        match &self.kind {
            InspectorPickerSceneKind::Select { trigger, .. }
            | InspectorPickerSceneKind::Asset { trigger, .. } => *trigger,
            InspectorPickerSceneKind::Color(scene) => scene.trigger,
        }
    }

    /// Returns the overlay bounds used by paint, hit testing, and semantics.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        match &self.kind {
            InspectorPickerSceneKind::Select { scene, .. }
            | InspectorPickerSceneKind::Asset { scene, .. } => scene
                .surfaces()
                .first()
                .map_or(Rect::ZERO, |surface| surface.entry().rect),
            InspectorPickerSceneKind::Color(scene) => scene.bounds,
        }
    }

    /// Adds this picker above lower application UI in one pointer plan.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
    ) -> PointerOrder {
        match &self.kind {
            InspectorPickerSceneKind::Select { scene, .. }
            | InspectorPickerSceneKind::Asset { scene, .. } => {
                scene.declare_pointer_targets(plan, first_order)
            }
            InspectorPickerSceneKind::Color(scene) => {
                let mut ordinal = first_order.raw();
                plan.capture_lower_layers(PointerOrder::new(ordinal));
                ordinal = ordinal.saturating_add(1);
                plan.blocker(scene.bounds, PointerOrder::new(ordinal));
                ordinal = ordinal.saturating_add(1);
                plan.with_clip(scene.bounds, |plan| {
                    for control in scene.controls() {
                        if control.rect.intersection(scene.bounds).is_some() {
                            plan.target(PointerTarget::new(
                                control.id,
                                control.rect,
                                PointerOrder::new(ordinal),
                            ));
                            ordinal = ordinal.saturating_add(1);
                        }
                    }
                });
                PointerOrder::new(ordinal)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InspectorPickerSceneKind {
    Select {
        trigger: WidgetId,
        overlay_id: OverlayId,
        scene: OverlayScene,
    },
    Color(ColorPickerScene),
    Asset {
        trigger: WidgetId,
        overlay_id: OverlayId,
        scene: OverlayScene,
        identities: BTreeMap<DropdownItemId, String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum InspectorPickerSession {
    Scene(InspectorPickerScene),
    Path(PathPickerSession),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PathPickerSession {
    request: PathPickerRequest,
    emitted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ColorPickerScene {
    pub(crate) trigger: WidgetId,
    pub(crate) overlay_id: OverlayId,
    pub(crate) root: WidgetId,
    pub(crate) bounds: Rect,
    pub(crate) original: Color,
    pub(crate) draft: Color,
}

impl ColorPickerScene {
    fn new(trigger: WidgetId, overlay_id: OverlayId, bounds: Rect, color: Color) -> Self {
        let color = sanitize_color(color);
        Self {
            trigger,
            overlay_id,
            root: trigger.child(("inspector-color-picker", overlay_id.raw())),
            bounds,
            original: color,
            draft: color,
        }
    }

    pub(crate) fn controls(&self) -> Vec<ColorPickerControl> {
        let inset = 4.0;
        let row_height = 28.0;
        let button_width = 24.0_f32.min((self.bounds.width - inset * 2.0).max(0.0));
        let mut controls = Vec::with_capacity(10);
        for (channel, offset) in ColorPickerChannel::ALL
            .into_iter()
            .zip([0.0_f32, 1.0, 2.0, 3.0])
        {
            let y = self.bounds.y + inset + offset * row_height;
            let decrement = Rect::new(
                self.bounds.max_x() - inset - button_width * 2.0 - 4.0,
                y,
                button_width,
                row_height,
            );
            let increment = Rect::new(
                self.bounds.max_x() - inset - button_width,
                y,
                button_width,
                row_height,
            );
            controls.push(ColorPickerControl {
                id: self.root.child((channel.label(), "decrement")),
                rect: decrement,
                label: format!("Decrease {}", channel.label()),
                action: ColorPickerAction::Adjust(channel, -0.05),
            });
            controls.push(ColorPickerControl {
                id: self.root.child((channel.label(), "increment")),
                rect: increment,
                label: format!("Increase {}", channel.label()),
                action: ColorPickerAction::Adjust(channel, 0.05),
            });
        }
        let action_y = self.bounds.max_y() - inset - row_height;
        let action_width = ((self.bounds.width - inset * 2.0 - 4.0) * 0.5).max(0.0);
        controls.push(ColorPickerControl {
            id: self.root.child("cancel"),
            rect: Rect::new(self.bounds.x + inset, action_y, action_width, row_height),
            label: "Cancel".to_owned(),
            action: ColorPickerAction::Cancel,
        });
        controls.push(ColorPickerControl {
            id: self.root.child("apply"),
            rect: Rect::new(
                self.bounds.x + inset + action_width + 4.0,
                action_y,
                action_width,
                row_height,
            ),
            label: "Apply".to_owned(),
            action: ColorPickerAction::Apply,
        });
        controls
    }

    pub(crate) fn adjust(&mut self, channel: ColorPickerChannel, delta: f32) {
        let value = channel.value(self.draft);
        channel.set(&mut self.draft, (value + delta).clamp(0.0, 1.0));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorPickerChannel {
    Red,
    Green,
    Blue,
    Alpha,
}

impl ColorPickerChannel {
    pub(crate) const ALL: [Self; 4] = [Self::Red, Self::Green, Self::Blue, Self::Alpha];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Alpha => "Alpha",
        }
    }

    pub(crate) const fn value(self, color: Color) -> f32 {
        match self {
            Self::Red => color.r,
            Self::Green => color.g,
            Self::Blue => color.b,
            Self::Alpha => color.a,
        }
    }

    fn set(self, color: &mut Color, value: f32) {
        match self {
            Self::Red => color.r = value,
            Self::Green => color.g = value,
            Self::Blue => color.b = value,
            Self::Alpha => color.a = value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ColorPickerAction {
    Adjust(ColorPickerChannel, f32),
    Apply,
    Cancel,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ColorPickerControl {
    pub(crate) id: WidgetId,
    pub(crate) rect: Rect,
    pub(crate) label: String,
    pub(crate) action: ColorPickerAction,
}

fn dropdown_scene(
    kind: InspectorPickerKind,
    trigger: WidgetId,
    overlay_id: OverlayId,
    bounds: Rect,
    label: String,
    model: DropdownModel,
    identities: BTreeMap<DropdownItemId, String>,
) -> InspectorPickerScene {
    let overlay = DropdownOverlay::new(
        OverlayEntry::new(overlay_id, OverlayKind::Dropdown, bounds)
            .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
        trigger,
        model,
    );
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::dropdown(label, overlay));
    InspectorPickerScene {
        kind: match kind {
            InspectorPickerKind::Select => InspectorPickerSceneKind::Select {
                trigger,
                overlay_id,
                scene,
            },
            InspectorPickerKind::Asset => InspectorPickerSceneKind::Asset {
                trigger,
                overlay_id,
                scene,
                identities,
            },
            InspectorPickerKind::Color | InspectorPickerKind::Path => {
                unreachable!("dropdown scenes are select or asset pickers")
            }
        },
        opened_frame: None,
    }
}

fn ensure_highlight(model: &mut DropdownModel) {
    if model.highlighted_id().is_none() {
        if let Some(selected) = model.selected_id() {
            let _ = model.set_highlighted_id(selected);
        }
        if model.highlighted_id().is_none() {
            let _ = model.highlight_first();
        }
    }
}

fn valid_dropdown_items(items: &[DropdownItem]) -> bool {
    items.iter().any(|item| item.enabled) && unique_dropdown_items(items)
}

fn unique_dropdown_items(items: &[DropdownItem]) -> bool {
    let mut ids = BTreeSet::new();
    items.iter().all(|item| ids.insert(item.id))
}

fn valid_asset_items(items: &[AssetPickerItem]) -> bool {
    items.iter().any(|item| item.enabled) && unique_asset_items(items)
}

fn unique_asset_items(items: &[AssetPickerItem]) -> bool {
    let mut ids = BTreeSet::new();
    let mut identities = BTreeSet::new();
    items.iter().all(|item| {
        !item.identity.is_empty()
            && ids.insert(item.id)
            && identities.insert(item.identity.as_str())
    })
}

fn valid_overlay_bounds(bounds: Rect) -> bool {
    bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.width > 0.0
        && bounds.height > 0.0
}

fn valid_color_overlay_bounds(bounds: Rect) -> bool {
    valid_overlay_bounds(bounds)
        && bounds.width >= MIN_COLOR_PICKER_WIDTH
        && bounds.height >= MIN_COLOR_PICKER_HEIGHT
}

fn sanitize_color(color: Color) -> Color {
    Color::rgba(
        sanitize_channel(color.r),
        sanitize_channel(color.g),
        sanitize_channel(color.b),
        sanitize_channel(color.a),
    )
}

fn sanitize_channel(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
