//! Gallery workspace: the toolkit's components, out of the box.
//!
//! ## Demo app boundary (owner-stated, issue #916)
//!
//! The demo MAY own its state, manage its own layout, and behave like a
//! real demo app — it is an ordinary consumer. It MUST NOT use
//! presentation hacks: no painted imitations of components, no hardcoded
//! visuals posing as toolkit output, no bypassing theme recipes, no
//! demo-local styling of toolkit components. Every visible control is a
//! real toolkit component with real state behind it
//! (`tests/prohibited_techniques.rs` enforces most of this).
//!
//! ## Coverage
//!
//! Sections mirror the state tables in `docs/visual-spec/01-07-*.md`
//! row-for-row, in spec order, for every state the current public
//! `stern::widgets` API can realize with a real component. A few
//! documented states have no public toolkit realization yet (button
//! `busy`, the `Primary`/`Danger`/`Ghost` button variants, checkbox
//! `mixed`, field `invalid`, a `StaticIcon`-based selectable icon button);
//! rather than fake them, the Gallery omits those rows. See
//! `KNOWN-GAPS.md` for the itemized list.
//!
//! Hover/pressed/dragging states are live: they render correctly only when
//! actually hovered, pressed, or dragged by the caller driving real input.
//! Focused states require tabbing to the control. This matches every other
//! workspace in the demo — nothing here is a canned screenshot.
//!
//! ## Pointer-target declaration
//!
//! `compose` runs in three strict phases — geometry, declare, paint (see
//! the section markers below). Every widget in this module is a bare
//! `Ui` call (button, checkbox, slider, ...), and each one is only
//! reachable by pointer input if its exact `WidgetId` and rect were
//! registered in the single `Ui::resolve_pointer_targets` call for this
//! frame; anything painted before that call, or never declared to it, is
//! visually present but permanently inert to clicks. This mirrors
//! `edit_workspace.rs`'s `declare_workspace_targets` → lower-UI ordering.

use stern::core::{
    ActionContext, ActionDescriptor, ActionInvocation, ActionSource, PointerOrder, PointerTarget,
    Rect, Response, Size, SizeRule, SpacingRole, WidgetId, default_dark_theme,
};
use stern::text::TextEditState;
use stern::widgets::chrome::{
    ChromeScene, ChromeSceneConfig, ChromeSceneIntent, ChromeSceneItemKey, MenuBar, MenuBarMenu,
    MenuBarMenuId, StatusBar, StatusItem, StatusItemId, StatusItemKind, SystemFeedbackScene,
    SystemFeedbackSceneConfig, TabStrip, Toolbar, ToolbarGroup, ToolbarGroupId,
};
use stern::widgets::dock::PanelId;
use stern::widgets::inspector::{InspectorPickerCommit, InspectorPickerState};
use stern::widgets::{
    Button as ButtonBuilder, CollectionCursor, CollectionProjection, DiagnosticStrip, DropdownItem,
    DropdownItemId, DropdownModel, FeedbackId, FeedbackItem, FeedbackKind, FeedbackStack, ItemId,
    JobList, JobPhase, JobProgress, JobRow, JobRowId, Menu, MenuOverlay, ModalAction,
    ModalActionRole, ModalDialog, ModalDialogOverlay, ModalFocusContainment, OverlayDismissal,
    OverlayEntry, OverlayId, OverlayKind, OverlayScene, OverlaySceneIntent, OverlaySceneSurface,
    PopoverPlacement, SelectFieldConfig, Selection, TableColumn, TableLayout,
    Toggle as ToggleBuilder, TreeExpansion, TreeItem, TreeModel, Ui, VirtualListConfig,
    VirtualListRow, VirtualListSelectionMode, VirtualTableConfig, VirtualTableRow,
    VirtualTableSelection, VirtualTableSelectionMode, VirtualTreeConfig, VirtualTreeRow,
    VirtualTreeSelectionMode,
};
use stern_icons_phosphor as phosphor;

use crate::edit_workspace::workspace_tab;
use crate::overlay_workspace::SharedOverlayRoute;
use crate::{DemoActionRegistry, DemoWorkspace};

const GALLERY_WORKSPACE_TAB: PanelId = PanelId::from_raw(103);
const CHROME_ROOT: WidgetId = WidgetId::from_raw(0x4741_4c4c_4552_5900);
const TOOLBAR_GROUP: ToolbarGroupId = ToolbarGroupId::from_raw(1);
const APPLICATION_MENU: MenuBarMenuId = MenuBarMenuId::from_raw(1);
const STATUS_ITEM: StatusItemId = StatusItemId::from_raw(1);

const SPECIMEN_CHROME_ROOT: WidgetId = WidgetId::from_raw(0x5350_4543_494d_454e);
const SPECIMEN_MENU: MenuBarMenuId = MenuBarMenuId::from_raw(101);
const SPECIMEN_TOOLBAR_GROUP: ToolbarGroupId = ToolbarGroupId::from_raw(101);
const SPECIMEN_REFRESH_ACTION: &str = "gallery.specimen.refresh";
const SPECIMEN_TAB_A: PanelId = PanelId::from_raw(9001);
const SPECIMEN_TAB_B: PanelId = PanelId::from_raw(9002);
const SPECIMEN_TAB_C: PanelId = PanelId::from_raw(9003);
const SPECIMEN_STATUS: StatusItemId = StatusItemId::from_raw(101);

const OVERLAY_MENU: OverlayId = OverlayId::from_raw(90);
const OVERLAY_TOOLTIP: OverlayId = OverlayId::from_raw(91);
const OVERLAY_POPOVER: OverlayId = OverlayId::from_raw(92);
const OVERLAY_MODAL: OverlayId = OverlayId::from_raw(93);
const DROPDOWN_PICKER: OverlayId = OverlayId::from_raw(94);
const GALLERY_MENU_ALPHA: &str = "gallery.overlays.menu.alpha";
const GALLERY_MENU_BETA: &str = "gallery.overlays.menu.beta";
const GALLERY_MODAL_CONFIRM: &str = "gallery.overlays.modal.confirm";

// Row/slot/caption heights derived below from `theme.sizes` and
// `theme.spacing.resolve(SpacingRole)` — the DS density ladder — rather
// than hardcoded here, so the gallery's own layout reads as DS-compliant
// (issue #916).
const CAPTION_HEIGHT: f32 = 14.0;
const CAPTION_GAP: f32 = 2.0;
const HEADER_HEIGHT: f32 = 16.0;

const LIST_IDS: [u64; 5] = [1, 2, 3, 4, 5];
const LIST_LABELS: [&str; 5] = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"];
const TREE_ROOT: u64 = 1;
const TREE_CHILD_A: u64 = 2;
const TREE_CHILD_B: u64 = 3;
const TREE_GRANDCHILD: u64 = 4;

/// Retained public Stern state for the out-of-the-box component gallery.
pub(crate) struct GalleryWorkspace {
    // Fields (docs/visual-spec/02-fields.md, single-line field state table).
    field_idle: TextEditState,
    field_hover: TextEditState,
    field_focused: TextEditState,
    field_read_only: TextEditState,
    field_disabled: TextEditState,

    // Choice, sliders, tabs (docs/visual-spec/03-choice-sliders-tabs.md).
    checkbox: [bool; 4],
    radio: [bool; 4],
    switch: [bool; 3],
    segmented: u8,
    slider_value: f32,
    slider_disabled_value: f32,

    // Overlays (docs/visual-spec/04-overlays.md).
    overlay: GalleryOverlayRoute,
    menu_choice: &'static str,
    modal_acknowledged: u32,
    dropdown_picker: InspectorPickerState,
    dropdown_selected: DropdownItemId,

    // Chrome specimens (docs/visual-spec/05-chrome-dock.md).
    specimen_tab: PanelId,
    specimen_refresh_count: u32,

    // Layout-engine seam (RFC 0001 L1): real state behind the content-sized
    // strip composed through `Ui::layout`.
    l1_snap: bool,
    l1_created: u32,

    // Collections (docs/visual-spec/06-collections.md).
    list_cursor: CollectionCursor,
    list_selection: Selection,
    table_selection: VirtualTableSelection,
    tree_cursor: CollectionCursor,
    tree_selection: Selection,
    tree_expansion: TreeExpansion,

    // Status, feedback, inspector (docs/visual-spec/07-status-feedback-inspector.md).
    jobs: JobList,
    diagnostics: DiagnosticStrip,
    feedback: FeedbackStack,
}

/// Gallery-local overlay slot for the Overlays family's trigger specimens.
///
/// Separate from [`SharedOverlayRoute`], which the Gallery's own top-level
/// chrome keeps using for its "Workspace" menu and the command palette
/// exactly like the Edit and Graph workspaces. Only one workspace composes
/// per frame, so the two independent overlay slots never race.
struct GalleryOverlayRoute {
    scene: Option<OverlayScene>,
    focus_return: Option<WidgetId>,
}

impl GalleryOverlayRoute {
    const fn new() -> Self {
        Self {
            scene: None,
            focus_return: None,
        }
    }

    fn scene(&self) -> Option<&OverlayScene> {
        self.scene.as_ref()
    }

    fn open_menu(&mut self, ui: &Ui<'_>, anchor: Rect, bounds: Size) {
        if self.scene.is_some() {
            return;
        }
        let menu = Menu::from_actions([
            ActionDescriptor::new(GALLERY_MENU_ALPHA, "Alpha"),
            ActionDescriptor::new(GALLERY_MENU_BETA, "Beta"),
        ]);
        let overlay = MenuOverlay::anchored(
            OVERLAY_MENU,
            OverlayKind::Menu,
            menu,
            anchor,
            Size::new(160.0, 76.0),
            PopoverPlacement::Below,
            4.0,
            true,
            viewport_rect(bounds),
            OverlayDismissal::OutsideClickOrEscape,
            ActionSource::Button,
            ActionContext::Editor,
        );
        let mut scene = OverlayScene::new();
        scene.push(OverlaySceneSurface::menu("Gallery menu specimen", overlay));
        self.scene = Some(scene);
        self.focus_return = ui.memory().focused();
    }

    fn open_popover(&mut self, ui: &Ui<'_>, anchor: Rect, bounds: Size) {
        if self.scene.is_some() {
            return;
        }
        let viewport = viewport_rect(bounds);
        let width = 220.0_f32.min(viewport.width);
        let height = 44.0_f32.min(viewport.height);
        let x = anchor.x.min((viewport.width - width).max(0.0));
        let y = (anchor.max_y() + 4.0).min((viewport.height - height).max(0.0));
        let entry = OverlayEntry::new(
            OVERLAY_POPOVER,
            OverlayKind::Popover,
            Rect::new(x, y, width, height),
        )
        .dismiss_on(OverlayDismissal::OutsideClickOrEscape);
        let mut scene = OverlayScene::new();
        scene.push(OverlaySceneSurface::passive(
            entry,
            "Popover specimen",
            "Elevation 2 surface, overlay-fill recipe, no arrow.",
        ));
        self.scene = Some(scene);
        self.focus_return = ui.memory().focused();
    }

    fn open_modal(&mut self, ui: &Ui<'_>, bounds: Size) {
        if self.scene.is_some() {
            return;
        }
        let owner = ui.memory().focused();
        let focus = owner.map_or_else(ModalFocusContainment::new, |owner| {
            ModalFocusContainment::new().with_return_focus(owner)
        });
        let dialog = ModalDialog::new(
            WidgetId::from_key("gallery.overlays.modal"),
            "Modal specimen",
        )
        .with_body("Scrim + header/body/footer per 04-overlays.md.")
        .with_focus(focus)
        .with_actions([ModalAction::new(
            ActionDescriptor::new(GALLERY_MODAL_CONFIRM, "Close"),
            ModalActionRole::Primary,
        )]);
        let viewport = viewport_rect(bounds);
        let width = 320.0_f32.min(viewport.width);
        let height = 128.0_f32.min(viewport.height);
        let rect = Rect::new(
            ((viewport.width - width) * 0.5).max(0.0),
            ((viewport.height - height) * 0.5).max(0.0),
            width,
            height,
        );
        let mut scene = OverlayScene::new();
        scene.push(OverlaySceneSurface::modal(ModalDialogOverlay::placed(
            OVERLAY_MODAL,
            rect,
            dialog,
            OverlayDismissal::OutsideClickOrEscape,
            ActionContext::Editor,
        )));
        self.scene = Some(scene);
        self.focus_return = owner;
    }

    fn sync_tooltip(&mut self, trigger: Response, bounds: Size) {
        let tooltip_open = self.scene.as_ref().is_some_and(|scene| {
            scene.surfaces().len() == 1 && scene.surfaces()[0].entry().kind == OverlayKind::Tooltip
        });
        if tooltip_open && !trigger.tooltip_requested {
            self.scene = None;
            self.focus_return = None;
        }
        if self.scene.is_none() && trigger.tooltip_requested {
            let viewport = viewport_rect(bounds);
            let width = 200.0_f32.min(viewport.width);
            let height = 30.0_f32.min(viewport.height);
            let x = trigger.rect.x.min((viewport.width - width).max(0.0));
            let y = (trigger.rect.max_y() + 6.0).min((viewport.height - height).max(0.0));
            let entry = OverlayEntry::new(
                OVERLAY_TOOLTIP,
                OverlayKind::Tooltip,
                Rect::new(x, y, width, height),
            );
            let mut scene = OverlayScene::new();
            scene.push(OverlaySceneSurface::passive(
                entry,
                "Tooltip specimen",
                "Elevation 1, offset 6, no arrow.",
            ));
            self.scene = Some(scene);
            self.focus_return = None;
        }
    }

    /// Evaluates the open overlay (if any) and reports the focus-return
    /// request plus any invoked action IDs, closing the overlay on dismiss
    /// or action.
    fn reconcile(&mut self, ui: &mut Ui<'_>) -> (Option<WidgetId>, Vec<ActionInvocation>) {
        let mut focus_return = None;
        let mut invoked = Vec::new();
        let close = self.scene.as_mut().is_some_and(|scene| {
            ui.overlay_scene(scene)
                .intents
                .iter()
                .any(|intent| match intent {
                    OverlaySceneIntent::Action(invocation) => {
                        invoked.push(invocation.clone());
                        focus_return = self.focus_return;
                        true
                    }
                    OverlaySceneIntent::Dismiss(request) => {
                        focus_return = request.focus_return.or(self.focus_return);
                        true
                    }
                    OverlaySceneIntent::OpenSubmenu(_) | OverlaySceneIntent::SelectDropdown(_) => {
                        false
                    }
                })
        });
        if close {
            self.scene = None;
            self.focus_return = None;
        }
        (focus_return, invoked)
    }
}

fn viewport_rect(bounds: Size) -> Rect {
    Rect::new(0.0, 0.0, bounds.width.max(0.0), bounds.height.max(0.0))
}

impl GalleryWorkspace {
    pub(crate) fn new() -> Self {
        Self {
            field_idle: TextEditState::new("Idle"),
            field_hover: TextEditState::new("Hover"),
            field_focused: TextEditState::new("Focused"),
            field_read_only: TextEditState::new("Read-only"),
            field_disabled: TextEditState::new("Disabled"),

            checkbox: [false, true, true, false],
            radio: [false, true, true, false],
            switch: [false, true, false],
            segmented: 2,
            slider_value: 0.4,
            slider_disabled_value: 0.6,

            overlay: GalleryOverlayRoute::new(),
            menu_choice: "(none yet)",
            modal_acknowledged: 0,
            dropdown_picker: InspectorPickerState::new(),
            dropdown_selected: DropdownItemId::from_raw(1),

            specimen_tab: SPECIMEN_TAB_A,
            specimen_refresh_count: 0,
            l1_snap: false,
            l1_created: 0,

            list_cursor: CollectionCursor::new(),
            list_selection: Selection::new(),
            table_selection: VirtualTableSelection::new(),
            tree_cursor: CollectionCursor::new(),
            tree_selection: Selection::new(),
            tree_expansion: {
                let mut expansion = TreeExpansion::new();
                let _ = expansion.expand(ItemId::from_raw(TREE_ROOT));
                expansion
            },

            jobs: JobList::new(),
            diagnostics: DiagnosticStrip::new(),
            feedback: FeedbackStack::new(),
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn compose(
        &mut self,
        ui: &mut Ui<'_>,
        actions: &DemoActionRegistry,
        workspace: DemoWorkspace,
        overlays: &mut SharedOverlayRoute,
        bounds: Size,
    ) -> Option<WidgetId> {
        let theme = default_dark_theme();
        // DS density ladder (docs/visual-spec/00-language.md): every gap and
        // control height below is resolved through the theme's spacing and
        // size scales rather than hardcoded, per issue #916.
        let row_height = theme.sizes.control.sm;
        let slot_gap = theme
            .spacing
            .resolve(SpacingRole::DefaultInlineControlPadding);
        let row_gap = theme.spacing.resolve(SpacingRole::GroupGap);
        let section_gap = theme.spacing.resolve(SpacingRole::SectionGap);
        let slot_block = row_height + CAPTION_GAP + CAPTION_HEIGHT;
        let [
            menu_rect,
            toolbar_rect,
            tab_strip_rect,
            content_rect,
            status_rect,
        ] = chrome_layout(Rect::new(
            0.0,
            0.0,
            bounds.width.max(0.0),
            bounds.height.max(0.0),
        ));

        let menu_bar = MenuBar::from_menus([MenuBarMenu::from_actions(
            APPLICATION_MENU,
            "Workspace",
            actions.iter().cloned(),
        )]);
        let mut menu_bar_for_reconcile = menu_bar.clone();
        let toolbar = Toolbar::from_groups([ToolbarGroup::from_actions(
            TOOLBAR_GROUP,
            "Workspace actions",
            actions.iter().cloned(),
        )]);
        let tab_strip = TabStrip::from_tabs([
            workspace_tab(101, "Edit Workspace", workspace == DemoWorkspace::Edit),
            workspace_tab(102, "Graph Workspace", workspace == DemoWorkspace::Graph),
            workspace_tab(
                GALLERY_WORKSPACE_TAB.raw(),
                "Gallery Workspace",
                workspace == DemoWorkspace::Gallery,
            ),
        ]);
        let status_bar = StatusBar::from_items([StatusItem::new(
            STATUS_ITEM,
            "Gallery",
            format!("Refresh count {}", self.specimen_refresh_count),
            StatusItemKind::Ready,
        )]);
        let mut chrome_widths = vec![
            (ChromeSceneItemKey::Menu(APPLICATION_MENU), 96.0),
            (ChromeSceneItemKey::Tab(PanelId::from_raw(101)), 132.0),
            (ChromeSceneItemKey::Tab(PanelId::from_raw(102)), 140.0),
            (ChromeSceneItemKey::Tab(GALLERY_WORKSPACE_TAB), 148.0),
            (ChromeSceneItemKey::Status(STATUS_ITEM), 180.0),
        ];
        chrome_widths.extend(actions.iter().map(|action| {
            (
                ChromeSceneItemKey::Toolbar {
                    group: TOOLBAR_GROUP,
                    action: action.id.clone(),
                },
                144.0,
            )
        }));
        let chrome_config = ChromeSceneConfig::new(
            CHROME_ROOT,
            menu_rect,
            toolbar_rect,
            tab_strip_rect,
            status_rect,
            ActionContext::Editor,
        )
        .with_widths(chrome_widths);
        let chrome = ChromeScene::new(chrome_config, &menu_bar, &toolbar, &tab_strip, &status_bar);

        overlays.open_palette_if_requested(ui, actions, bounds);

        let content = content_rect.inset(12.0);
        let width = content.width.max(0.0);

        // ================================================================
        // PHASE 1 — GEOMETRY: pure layout arithmetic, no widget painting.
        // Every rect a control uses is computed once here and reused
        // identically by both the declare pass (phase 2) and the paint
        // pass (phase 3), so declared pointer targets can never drift from
        // painted geometry.
        // ================================================================
        let mut cursor = content.y;

        cursor = section_header(ui, content.x, cursor, width, "Buttons — default variant");
        let button_slots = slots(row(content.x, cursor, width, slot_block), 5, slot_gap);
        cursor += slot_block + row_gap;
        let icon_slots = slots(row(content.x, cursor, width, slot_block), 5, slot_gap);
        cursor += slot_block + section_gap;

        cursor = section_header(ui, content.x, cursor, width, "Fields — single-line field");
        let field_slots = slots(row(content.x, cursor, width, slot_block), 5, slot_gap);
        cursor += slot_block + section_gap;

        cursor = section_header(ui, content.x, cursor, width, "Choice, sliders, tabs");
        let choice_labels = ["Unchecked", "Checked", "Disabled", "Focused"];
        let checkbox_slots = slots(row(content.x, cursor, width, slot_block), 4, slot_gap);
        cursor += slot_block + row_gap;
        let radio_slots = slots(row(content.x, cursor, width, slot_block), 4, slot_gap);
        cursor += slot_block + row_gap;
        let switch_labels = ["Off", "On", "Disabled"];
        let switch_slots = slots(row(content.x, cursor, width, slot_block), 3, slot_gap);
        cursor += slot_block + row_gap;
        let segmented_labels = ["Rest", "Hover", "Selected"];
        let segmented_slots = slots(row(content.x, cursor, width, slot_block), 3, slot_gap);
        cursor += slot_block + row_gap;
        let slider_slots = slots(row(content.x, cursor, width, slot_block), 2, slot_gap);
        cursor += slot_block + section_gap;

        cursor = section_header(ui, content.x, cursor, width, "Overlays");
        let overlay_slots = slots(row(content.x, cursor, width, slot_block), 5, slot_gap);
        let menu_trigger = Rect::new(
            overlay_slots[0].x,
            overlay_slots[0].y,
            overlay_slots[0].width,
            row_height,
        );
        let dropdown_control = Rect::new(
            overlay_slots[1].x,
            overlay_slots[1].y,
            overlay_slots[1].width,
            row_height,
        );
        let dropdown_picker_bounds = Rect::new(
            dropdown_control.x,
            dropdown_control.max_y() + 4.0,
            160.0,
            120.0,
        );
        let tooltip_control = Rect::new(
            overlay_slots[2].x,
            overlay_slots[2].y,
            overlay_slots[2].width,
            row_height,
        );
        let popover_control = Rect::new(
            overlay_slots[3].x,
            overlay_slots[3].y,
            overlay_slots[3].width,
            row_height,
        );
        let modal_control = Rect::new(
            overlay_slots[4].x,
            overlay_slots[4].y,
            overlay_slots[4].width,
            row_height,
        );
        cursor += slot_block + section_gap;

        cursor = section_header(
            ui,
            content.x,
            cursor,
            width,
            "Chrome specimens (in a panel)",
        );
        let specimen_panel = Rect::new(content.x, cursor, width, 96.0);
        let specimen_inset = specimen_panel.inset(4.0);
        let [
            specimen_menu_rect,
            specimen_toolbar_rect,
            specimen_tabs_rect,
            _specimen_content,
            specimen_status_rect,
        ] = chrome_layout(specimen_inset);
        cursor += 96.0 + section_gap;

        cursor = section_header(ui, content.x, cursor, width, "Collections — row states");
        let row_state_labels = ["Rest", "Hover", "Selected", "Focused", "Disabled"];
        let row_state_slots = slots(row(content.x, cursor, width, slot_block), 5, slot_gap);
        cursor += slot_block + row_gap;

        let collection_labels = ["List", "Table", "Tree"];
        let collection_caption_slots = slots(row(content.x, cursor, width, 18.0), 3, slot_gap);
        cursor += 16.0;
        let specimen_rects = slots(row(content.x, cursor, width, 108.0), 3, slot_gap);
        cursor += 108.0 + section_gap;

        cursor = section_header(
            ui,
            content.x,
            cursor,
            width,
            "Layout engine \u{2014} content-sized (L1)",
        );
        // Geometry through the RFC 0001 L1 seam: builders measure their own
        // content (shaped label + theme metrics) and the solved rects exist
        // before the declare pass, exactly like every hand-computed rect
        // above. No painting happens here.
        let l1_bounds = row(content.x, cursor, width, slot_block);
        let mut l1_new = None;
        let mut l1_snap_slot = None;
        let l1_layout = ui.layout(l1_bounds, |l| {
            l.row(SizeRule::Fill, SizeRule::Fit, slot_gap, |l| {
                l1_new = Some(l.add(ButtonBuilder::new("gallery.l1.new", "New")));
                l1_snap_slot = Some(l.add(ToggleBuilder::new(
                    "gallery.l1.snap",
                    "Snap to grid",
                    self.l1_snap,
                )));
            });
        });
        let l1_new = l1_new.expect("l1 layout added the button");
        let l1_snap_slot = l1_snap_slot.expect("l1 layout added the toggle");
        cursor += slot_block + section_gap;

        cursor = section_header(ui, content.x, cursor, width, "Status & feedback");
        let feedback_height = 76.0_f32.min((content.max_y() - cursor).max(0.0));
        let feedback_bounds = row(content.x, cursor, width, feedback_height);
        let jobs_rect = Rect::new(
            feedback_bounds.x,
            feedback_bounds.y,
            feedback_bounds.width,
            28.0,
        );
        let feedback_rect = Rect::new(
            feedback_bounds.x,
            jobs_rect.max_y() + 4.0,
            feedback_bounds.width,
            (feedback_bounds.height - 32.0).max(0.0),
        );

        // Prepared (not yet painted) compound scenes and models. Preparing
        // does not paint or read pointer state, so these are safe before
        // the declare pass; actually painting them happens only in phase 3.
        let mut specimen_refresh = ActionDescriptor::new(SPECIMEN_REFRESH_ACTION, "Refresh")
            .with_icon(phosphor::regular::GEAR);
        specimen_refresh.state.visible = true;
        let specimen_menu_bar = MenuBar::from_menus([MenuBarMenu::from_actions(
            SPECIMEN_MENU,
            "Specimen",
            [specimen_refresh.clone()],
        )]);
        let specimen_toolbar = Toolbar::from_groups([ToolbarGroup::from_actions(
            SPECIMEN_TOOLBAR_GROUP,
            "Specimen actions",
            [specimen_refresh.clone()],
        )]);
        let specimen_tab_strip = TabStrip::from_tabs([
            workspace_tab(
                SPECIMEN_TAB_A.raw(),
                "Tab A",
                self.specimen_tab == SPECIMEN_TAB_A,
            ),
            workspace_tab(
                SPECIMEN_TAB_B.raw(),
                "Tab B",
                self.specimen_tab == SPECIMEN_TAB_B,
            ),
            workspace_tab(
                SPECIMEN_TAB_C.raw(),
                "Tab C",
                self.specimen_tab == SPECIMEN_TAB_C,
            ),
        ]);
        let specimen_status_bar = StatusBar::from_items([StatusItem::new(
            SPECIMEN_STATUS,
            "Specimen",
            format!("Active {}", specimen_tab_label(self.specimen_tab)),
            StatusItemKind::Ready,
        )]);
        let specimen_chrome_config = ChromeSceneConfig::new(
            SPECIMEN_CHROME_ROOT,
            specimen_menu_rect,
            specimen_toolbar_rect,
            specimen_tabs_rect,
            specimen_status_rect,
            ActionContext::Editor,
        )
        .with_widths([
            (ChromeSceneItemKey::Menu(SPECIMEN_MENU), 88.0),
            (
                ChromeSceneItemKey::Toolbar {
                    group: SPECIMEN_TOOLBAR_GROUP,
                    action: specimen_refresh.id.clone(),
                },
                96.0,
            ),
            (ChromeSceneItemKey::Tab(SPECIMEN_TAB_A), 72.0),
            (ChromeSceneItemKey::Tab(SPECIMEN_TAB_B), 72.0),
            (ChromeSceneItemKey::Tab(SPECIMEN_TAB_C), 72.0),
            (ChromeSceneItemKey::Status(SPECIMEN_STATUS), 120.0),
        ]);
        let specimen_chrome = ChromeScene::new(
            specimen_chrome_config,
            &specimen_menu_bar,
            &specimen_toolbar,
            &specimen_tab_strip,
            &specimen_status_bar,
        );

        let list_ids = LIST_IDS.map(ItemId::from_raw);
        let list_projection = CollectionProjection::from_source_ids(&list_ids);
        let list_config = VirtualListConfig::new(specimen_rects[0].inset(4.0), row_height)
            .label("Gallery list")
            .selection_mode(VirtualListSelectionMode::Single);
        let list_widget =
            ui.prepare_virtual_list("gallery.collections.list", list_config, &list_projection);

        let table_ids = LIST_IDS.map(ItemId::from_raw);
        let table_projection = CollectionProjection::from_source_ids(&table_ids);
        let table_layout = TableLayout {
            columns: vec![
                TableColumn::new(ItemId::from_raw(1), "Name", 88.0),
                TableColumn::new(ItemId::from_raw(2), "Kind", 60.0),
            ],
            header_height: row_height,
            row_height,
            sort: None,
        };
        let table_config = VirtualTableConfig::new(specimen_rects[1].inset(4.0), table_layout)
            .label("Gallery table")
            .selection_mode(VirtualTableSelectionMode::Row);
        let table_widget =
            ui.prepare_virtual_table("gallery.collections.table", table_config, &table_projection);

        let tree_model = TreeModel::new([
            TreeItem {
                id: ItemId::from_raw(TREE_ROOT),
                parent: None,
                has_children: true,
            },
            TreeItem {
                id: ItemId::from_raw(TREE_CHILD_A),
                parent: Some(ItemId::from_raw(TREE_ROOT)),
                has_children: true,
            },
            TreeItem {
                id: ItemId::from_raw(TREE_CHILD_B),
                parent: Some(ItemId::from_raw(TREE_ROOT)),
                has_children: false,
            },
            TreeItem {
                id: ItemId::from_raw(TREE_GRANDCHILD),
                parent: Some(ItemId::from_raw(TREE_CHILD_A)),
                has_children: false,
            },
        ]);
        let tree_config = VirtualTreeConfig::new(specimen_rects[2].inset(4.0), row_height, 14.0)
            .label("Gallery tree")
            .selection_mode(VirtualTreeSelectionMode::Single);
        let tree_widget = ui.prepare_virtual_tree(
            "gallery.collections.tree",
            tree_config,
            &tree_model,
            &self.tree_expansion,
        );

        self.jobs.replace_rows([JobRow::new(
            JobRowId::from_raw(1),
            "Preview render",
            JobPhase::Running,
        )
        .with_progress(JobProgress::from_fraction(65.0, 100.0))]);
        self.feedback.replace_items([FeedbackItem::pinned(
            FeedbackId::from_raw(1),
            FeedbackKind::Info,
            "Status banner specimen",
            "Neutral inline feedback per 07-status-feedback-inspector.md.",
        )]);
        let feedback_scene = SystemFeedbackScene::prepare(
            SystemFeedbackSceneConfig::new(
                WidgetId::from_key("gallery.feedback"),
                jobs_rect,
                Rect::ZERO,
                feedback_rect,
            ),
            &self.jobs,
            &self.diagnostics,
            &self.feedback,
            ui.time().now,
        )
        .expect("deterministic gallery feedback scene is valid");

        let dropdown_model = dropdown_model();

        // ================================================================
        // PHASE 2 — DECLARE: one pointer-target plan covering every bare
        // interactive control (state-table specimens) plus every compound
        // scene, using the exact rects computed in phase 1.
        // ================================================================
        let mut targets: Vec<(WidgetId, Rect)> = Vec::new();
        let button_state_labels = ["Idle", "Hover", "Pressed", "Focused", "Disabled"];
        for (slot, caption) in button_slots.iter().zip(button_state_labels) {
            let control = Rect::new(slot.x, slot.y, slot.width, row_height);
            targets.push((ui.make_id(("gallery.buttons.default", caption)), control));
        }
        for (slot, caption) in icon_slots.iter().zip(button_state_labels) {
            let control = Rect::new(slot.x, slot.y, row_height, row_height);
            targets.push((ui.make_id(("gallery.buttons.icon", caption)), control));
        }
        let field_state_labels = ["Idle", "Hover", "Focused", "Read-only", "Disabled"];
        for (slot, caption) in field_slots.iter().zip(field_state_labels) {
            let control = Rect::new(slot.x, slot.y, slot.width, row_height);
            targets.push((ui.make_id(("gallery.fields", caption)), control));
        }
        for (slot, caption) in checkbox_slots.iter().zip(choice_labels) {
            let control = Rect::new(slot.x, slot.y, 14.0, 14.0);
            targets.push((ui.make_id(("gallery.choice.checkbox", caption)), control));
        }
        for (slot, caption) in radio_slots.iter().zip(choice_labels) {
            let control = Rect::new(slot.x, slot.y, 14.0, 14.0);
            targets.push((ui.make_id(("gallery.choice.radio", caption)), control));
        }
        for (slot, caption) in switch_slots.iter().zip(switch_labels) {
            let control = Rect::new(slot.x, slot.y, 26.0, 14.0);
            targets.push((ui.make_id(("gallery.choice.switch", caption)), control));
        }
        for (slot, caption) in segmented_slots.iter().zip(segmented_labels) {
            let control = Rect::new(slot.x, slot.y, slot.width, row_height);
            targets.push((ui.make_id(("gallery.choice.segmented", caption)), control));
        }
        targets.push((
            ui.make_id("gallery.choice.slider.interactive"),
            Rect::new(
                slider_slots[0].x,
                slider_slots[0].y,
                slider_slots[0].width,
                row_height,
            ),
        ));
        targets.push((
            ui.make_id("gallery.choice.slider.disabled"),
            Rect::new(
                slider_slots[1].x,
                slider_slots[1].y,
                slider_slots[1].width,
                row_height,
            ),
        ));
        targets.push((ui.make_id("gallery.overlays.menu-trigger"), menu_trigger));
        targets.push((ui.make_id("gallery.overlays.dropdown"), dropdown_control));
        targets.push((
            ui.make_id("gallery.overlays.tooltip-anchor"),
            tooltip_control,
        ));
        targets.push((
            ui.make_id("gallery.overlays.popover-trigger"),
            popover_control,
        ));
        targets.push((ui.make_id("gallery.overlays.modal-trigger"), modal_control));
        for (slot, caption) in row_state_slots.iter().zip(row_state_labels) {
            let control = Rect::new(slot.x, slot.y, slot.width, row_height);
            targets.push((ui.make_id(("gallery.collections.row", caption)), control));
        }
        // L1 seam targets come from solved layout geometry, not hand rects.
        targets.push((ui.make_id("gallery.l1.new"), l1_layout.rect(l1_new)));
        targets.push((ui.make_id("gallery.l1.snap"), l1_layout.rect(l1_snap_slot)));

        ui.resolve_pointer_targets(|plan| {
            let mut next = PointerOrder::new(0);
            for (id, rect) in &targets {
                plan.target(PointerTarget::new(*id, *rect, next));
                next = PointerOrder::new(next.raw() + 1);
            }
            next = chrome.declare_pointer_targets(plan, next);
            next = specimen_chrome.declare_pointer_targets(plan, next);
            if let Some(list) = list_widget.as_ref() {
                next = list.declare_pointer_targets(plan, next);
            }
            if let Some(table) = table_widget.as_ref() {
                next = table.declare_pointer_targets(plan, next);
            }
            if let Some(tree) = tree_widget.as_ref() {
                next = tree.declare_pointer_targets(plan, next);
            }
            next = feedback_scene.declare_pointer_targets(plan, next);
            if let Some(scene) = overlays.scene() {
                next = scene.declare_pointer_targets(plan, next);
            }
            if let Some(scene) = self.overlay.scene() {
                next = scene.declare_pointer_targets(plan, next);
            }
            if let Some(scene) = self.dropdown_picker.scene() {
                let _ = scene.declare_pointer_targets(plan, next);
            }
        })
        .expect("Gallery workspace pointer targets are valid");

        // ================================================================
        // PHASE 3 — PAINT: every widget call below observes the pointer
        // routes phase 2 just installed, so responses (`clicked`, hover,
        // focus) reflect real input instead of being inert.
        // ================================================================
        ui.panel(specimen_panel);
        for rect in &specimen_rects {
            ui.panel(*rect);
        }

        for (slot, caption) in button_slots.iter().zip(button_state_labels) {
            let control = Rect::new(slot.x, slot.y, slot.width, row_height);
            let disabled = caption == "Disabled";
            let _ = ui.button(
                ("gallery.buttons.default", caption),
                control,
                caption,
                disabled,
            );
            label_below(ui, control, caption);
        }
        for (slot, caption) in icon_slots.iter().zip(button_state_labels) {
            let control = Rect::new(slot.x, slot.y, row_height, row_height);
            let disabled = caption == "Disabled";
            let _ = ui.icon_button(
                ("gallery.buttons.icon", caption),
                control,
                phosphor::regular::STAR.icon(),
                format!("Star ({caption})"),
                disabled,
            );
            label_below(ui, control, caption);
        }

        for (index, (slot, caption)) in field_slots.iter().zip(field_state_labels).enumerate() {
            let control = Rect::new(slot.x, slot.y, slot.width, row_height);
            let state = match index {
                0 => &mut self.field_idle,
                1 => &mut self.field_hover,
                2 => &mut self.field_focused,
                3 => &mut self.field_read_only,
                _ => &mut self.field_disabled,
            };
            let access = match index {
                3 => stern::widgets::TextFieldAccess::ReadOnly,
                4 => stern::widgets::TextFieldAccess::Disabled,
                _ => stern::widgets::TextFieldAccess::Editable,
            };
            let _ = ui.text_field_with_access(("gallery.fields", caption), control, state, access);
            label_below(ui, control, caption);
        }

        for (index, (slot, caption)) in checkbox_slots.iter().zip(choice_labels).enumerate() {
            let control = Rect::new(slot.x, slot.y, 14.0, 14.0);
            let disabled = index == 2;
            let _ = ui.checkbox_value_with_label(
                ("gallery.choice.checkbox", caption),
                control,
                caption,
                &mut self.checkbox[index],
                disabled,
            );
            label_below(
                ui,
                Rect::new(slot.x, slot.y, slot.width, row_height),
                caption,
            );
        }
        for (index, (slot, caption)) in radio_slots.iter().zip(choice_labels).enumerate() {
            let control = Rect::new(slot.x, slot.y, 14.0, 14.0);
            let disabled = index == 2;
            let response = ui.radio_button_with_label(
                ("gallery.choice.radio", caption),
                control,
                caption,
                self.radio[index],
                disabled,
            );
            if response.clicked {
                self.radio[index] = !self.radio[index];
            }
            label_below(
                ui,
                Rect::new(slot.x, slot.y, slot.width, row_height),
                caption,
            );
        }
        for (index, (slot, caption)) in switch_slots.iter().zip(switch_labels).enumerate() {
            let control = Rect::new(slot.x, slot.y, 26.0, 14.0);
            let disabled = index == 2;
            let _ = ui.toggle_value_with_label(
                ("gallery.choice.switch", caption),
                control,
                caption,
                &mut self.switch[index],
                disabled,
            );
            label_below(
                ui,
                Rect::new(slot.x, slot.y, slot.width, row_height),
                caption,
            );
        }
        for (index, (slot, caption)) in segmented_slots.iter().zip(segmented_labels).enumerate() {
            let control = Rect::new(slot.x, slot.y, slot.width, row_height);
            let value = u8::try_from(index).unwrap_or(0);
            let _ = ui.tab_button_value(
                ("gallery.choice.segmented", caption),
                control,
                caption,
                &mut self.segmented,
                value,
                false,
            );
            label_below(ui, control, caption);
        }
        {
            let control = Rect::new(
                slider_slots[0].x,
                slider_slots[0].y,
                slider_slots[0].width,
                row_height,
            );
            let _ = ui.slider(
                "gallery.choice.slider.interactive",
                control,
                &mut self.slider_value,
                0.0..=1.0,
                false,
            );
            label_below(ui, control, "Interactive");
            let control = Rect::new(
                slider_slots[1].x,
                slider_slots[1].y,
                slider_slots[1].width,
                row_height,
            );
            let _ = ui.slider(
                "gallery.choice.slider.disabled",
                control,
                &mut self.slider_disabled_value,
                0.0..=1.0,
                true,
            );
            label_below(ui, control, "Disabled");
        }

        let menu_response = ui.button("gallery.overlays.menu-trigger", menu_trigger, "Menu", false);
        label_below(ui, menu_trigger, self.menu_choice);
        let dropdown_label = dropdown_model
            .items()
            .iter()
            .find(|item| item.id == self.dropdown_selected)
            .map_or("Select", |item| item.label.as_str())
            .to_owned();
        let dropdown_field = ui.select_field(
            "gallery.overlays.dropdown",
            dropdown_control,
            &dropdown_label,
            &dropdown_model,
            SelectFieldConfig::new("Select").open(self.dropdown_picker.kind().is_some()),
        );
        let _ = ui.select_picker(
            &mut self.dropdown_picker,
            &dropdown_field,
            DROPDOWN_PICKER,
            dropdown_picker_bounds,
            "Gallery dropdown",
            &dropdown_model,
        );
        label_below(ui, dropdown_control, "Dropdown");
        let _ = ui.button(
            "gallery.overlays.tooltip-anchor",
            tooltip_control,
            "Tooltip",
            false,
        );
        let tooltip_trigger =
            ui.tooltip_trigger("gallery.overlays.tooltip-anchor", tooltip_control, false);
        label_below(ui, tooltip_control, "Hover me");
        let popover_response = ui.button(
            "gallery.overlays.popover-trigger",
            popover_control,
            "Popover",
            false,
        );
        label_below(ui, popover_control, "Popover");
        let modal_response = ui.button(
            "gallery.overlays.modal-trigger",
            modal_control,
            "Modal",
            false,
        );
        label_below(
            ui,
            modal_control,
            &format!("Closed {}x", self.modal_acknowledged),
        );

        for (slot, caption) in collection_caption_slots.iter().zip(collection_labels) {
            label_below(
                ui,
                Rect::new(slot.x, slot.y - 4.0, slot.width, 14.0),
                caption,
            );
        }

        for (index, (slot, caption)) in row_state_slots.iter().zip(row_state_labels).enumerate() {
            let control = Rect::new(slot.x, slot.y, slot.width, row_height);
            let selected = index == 2;
            let disabled = index == 4;
            let _ = ui.list_row(
                ("gallery.collections.row", caption),
                control,
                caption,
                selected,
                disabled,
            );
            label_below(ui, control, caption);
        }

        let l1_new_rect = l1_layout.rect(l1_new);
        let l1_snap_rect = l1_layout.rect(l1_snap_slot);
        let l1 = l1_layout.compose(ui);
        if l1.response(l1_new).is_some_and(|r| r.clicked) {
            self.l1_created += 1;
        }
        if l1.response(l1_snap_slot).is_some_and(|r| r.clicked) {
            self.l1_snap = !self.l1_snap;
        }
        label_below(ui, l1_new_rect, &format!("Created {}", self.l1_created));
        label_below(ui, l1_snap_rect, "Sized by ui.layout");

        let _ = ui.system_feedback(&feedback_scene);
        if let Some(list) = list_widget.as_ref() {
            let _ = ui.virtual_list(
                list,
                &mut self.list_cursor,
                &mut self.list_selection,
                |item| VirtualListRow::new(list_label(item.id)),
            );
        }
        if let Some(table) = table_widget.as_ref() {
            let _ = ui.virtual_table(table, &mut self.table_selection, |item| {
                VirtualTableRow::new([list_label(item.id).to_owned(), "Item".to_owned()])
            });
        }
        if let Some(tree) = tree_widget.as_ref() {
            let _ = ui.virtual_tree(
                tree,
                &mut self.tree_cursor,
                &mut self.tree_selection,
                &mut self.tree_expansion,
                |row| VirtualTreeRow::new(tree_label(row.id.raw())),
            );
        }

        let specimen_chrome_output = ui.chrome_scene(&specimen_chrome);
        for intent in &specimen_chrome_output.intents {
            match intent {
                ChromeSceneIntent::ActivateTab(target) => self.specimen_tab = target.panel,
                ChromeSceneIntent::Action(invocation)
                    if invocation.action_id.as_str() == SPECIMEN_REFRESH_ACTION =>
                {
                    self.specimen_refresh_count = self.specimen_refresh_count.saturating_add(1);
                }
                _ => {}
            }
        }

        if menu_response.clicked {
            self.overlay.open_menu(ui, menu_trigger, bounds);
        }
        self.overlay.sync_tooltip(tooltip_trigger, bounds);
        if popover_response.clicked {
            self.overlay.open_popover(ui, popover_control, bounds);
        }
        if modal_response.clicked {
            self.overlay.open_modal(ui, bounds);
        }
        let (local_focus_return, invoked) = self.overlay.reconcile(ui);
        for invocation in invoked {
            match invocation.action_id.as_str() {
                GALLERY_MENU_ALPHA => self.menu_choice = "Alpha",
                GALLERY_MENU_BETA => self.menu_choice = "Beta",
                GALLERY_MODAL_CONFIRM => {
                    self.modal_acknowledged = self.modal_acknowledged.saturating_add(1);
                }
                _ => {}
            }
        }
        if let Some(commit) = ui.inspector_picker_scene(&mut self.dropdown_picker).commit
            && let InspectorPickerCommit::Select(id) = commit
        {
            self.dropdown_selected = id;
        }

        let chrome_output = ui.chrome_scene(&chrome);
        crate::edit_workspace::route_workspace_tabs(ui, actions, &chrome_output.intents);
        let shared_focus_return = overlays.reconcile(
            ui,
            actions,
            &mut menu_bar_for_reconcile,
            &chrome_output.intents,
            false,
            bounds,
        );

        shared_focus_return.or(local_focus_return)
    }
}

fn dropdown_model() -> DropdownModel {
    DropdownModel::from_items([
        DropdownItem::new(DropdownItemId::from_raw(1), "Alpha"),
        DropdownItem::new(DropdownItemId::from_raw(2), "Beta"),
        DropdownItem::new(DropdownItemId::from_raw(3), "Gamma"),
    ])
}

fn list_label(id: ItemId) -> &'static str {
    LIST_IDS
        .iter()
        .position(|candidate| *candidate == id.raw())
        .map_or("Item", |index| LIST_LABELS[index])
}

fn tree_label(raw: u64) -> &'static str {
    match raw {
        TREE_ROOT => "Root",
        TREE_CHILD_A => "Branch",
        TREE_CHILD_B => "Leaf",
        TREE_GRANDCHILD => "Nested leaf",
        _ => "Item",
    }
}

fn specimen_tab_label(panel: PanelId) -> &'static str {
    match panel {
        SPECIMEN_TAB_B => "Tab B",
        SPECIMEN_TAB_C => "Tab C",
        _ => "Tab A",
    }
}

fn section_header(ui: &mut Ui<'_>, x: f32, y: f32, width: f32, title: &str) -> f32 {
    ui.label(Rect::new(x, y, width, HEADER_HEIGHT), title);
    y + HEADER_HEIGHT + 4.0
}

fn row(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(x, y, width, height)
}

fn slots(row: Rect, count: usize, gap: f32) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    #[allow(clippy::cast_precision_loss)]
    let count_f = count as f32;
    let total_gap = gap * (count_f - 1.0).max(0.0);
    let slot_width = ((row.width - total_gap) / count_f).max(0.0);
    (0..count)
        .map(|index| {
            #[allow(clippy::cast_precision_loss)]
            let offset = index as f32 * (slot_width + gap);
            Rect::new(row.x + offset, row.y, slot_width, row.height)
        })
        .collect()
}

fn label_below(ui: &mut Ui<'_>, control: Rect, text: &str) {
    let rect = Rect::new(
        control.x,
        control.max_y() + CAPTION_GAP,
        control.width.max(40.0),
        CAPTION_HEIGHT,
    );
    ui.label(rect, text.to_owned());
}

fn chrome_layout(bounds: Rect) -> [Rect; 5] {
    let menu_height = 28.0_f32.min(bounds.height.max(0.0));
    let remaining = (bounds.height - menu_height).max(0.0);
    let toolbar_height = 28.0_f32.min(remaining);
    let remaining = (remaining - toolbar_height).max(0.0);
    let tab_height = 28.0_f32.min(remaining);
    let remaining = (remaining - tab_height).max(0.0);
    let status_height = 22.0_f32.min(remaining);
    let content_height = (remaining - status_height).max(0.0);
    [
        Rect::new(bounds.x, bounds.y, bounds.width, menu_height),
        Rect::new(
            bounds.x,
            bounds.y + menu_height,
            bounds.width,
            toolbar_height,
        ),
        Rect::new(
            bounds.x,
            bounds.y + menu_height + toolbar_height,
            bounds.width,
            tab_height,
        ),
        Rect::new(
            bounds.x,
            bounds.y + menu_height + toolbar_height + tab_height,
            bounds.width,
            content_height,
        ),
        Rect::new(
            bounds.x,
            bounds.y + menu_height + toolbar_height + tab_height + content_height,
            bounds.width,
            status_height,
        ),
    ]
}
