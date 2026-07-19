use stern::core::{ActionContext, ActionSource, Key, KeyState, Rect, Size, UiInput, WidgetId};
use stern::widgets::{
    ChromeSceneIntent, CommandPaletteOverlay, Menu, MenuBar, MenuBarOverlayRequest, MenuOverlay,
    ModalDialog, ModalDialogOverlay, ModalFocusContainment, OverlayDismissal, OverlayEntry,
    OverlayId, OverlayKind, OverlayScene, OverlaySceneIntent, OverlaySceneSurface,
    PopoverPlacement, Ui,
};

use crate::app_model::DemoColorOverlayNotice;
use crate::{DemoActionRegistry, DemoApplicationModel};

const APPLICATION_MENU_OVERLAY: OverlayId = OverlayId::from_raw(1);
const CONTEXT_MENU_OVERLAY: OverlayId = OverlayId::from_raw(2);
const COMMAND_PALETTE_OVERLAY: OverlayId = OverlayId::from_raw(3);
const COLOR_FAILURE_POPOVER: OverlayId = OverlayId::from_raw(6);
const COLOR_RECOVERY_MODAL: OverlayId = OverlayId::from_raw(7);

/// One DemoApp-owned route for transient UI shared by maintained workspaces.
pub(crate) struct SharedOverlayRoute {
    scene: Option<OverlayScene>,
    focus_return: Option<WidgetId>,
}

impl SharedOverlayRoute {
    pub(crate) const fn new() -> Self {
        Self {
            scene: None,
            focus_return: None,
        }
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.scene.is_some()
    }

    pub(crate) const fn scene(&self) -> Option<&OverlayScene> {
        self.scene.as_ref()
    }

    pub(crate) fn open_palette_if_requested(
        &mut self,
        ui: &Ui<'_>,
        actions: &DemoActionRegistry,
        bounds: Size,
    ) {
        if self.scene.is_none() && command_palette_requested(ui.input()) {
            self.scene = Some(command_palette_scene(actions, bounds));
            self.focus_return = ui.memory().focused();
        }
    }

    pub(crate) fn open_color_notice(
        &mut self,
        ui: &Ui<'_>,
        model: &mut DemoApplicationModel,
        bounds: Size,
    ) {
        if self.scene.is_some() {
            return;
        }
        let Some(notice) = model.take_color_overlay_notice() else {
            return;
        };
        let owner = ui.memory().focused();
        self.scene = Some(color_notice_scene(notice, bounds, owner));
        self.focus_return = owner;
    }

    pub(crate) fn reconcile(
        &mut self,
        ui: &mut Ui<'_>,
        actions: &DemoActionRegistry,
        menu_bar: &mut MenuBar,
        chrome_intents: &[ChromeSceneIntent],
        context_requested: bool,
        bounds: Size,
    ) -> Option<WidgetId> {
        let mut focus_return = None;
        let close_overlay = self.scene.as_mut().is_some_and(|scene| {
            ui.overlay_scene(scene)
                .intents
                .iter()
                .any(|intent| match intent {
                    OverlaySceneIntent::Action(_) => {
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
        if close_overlay {
            self.scene = None;
            self.focus_return = None;
        }
        if self.scene.is_none() {
            if let Some((menu, anchor)) = chrome_intents.iter().find_map(|intent| {
                let ChromeSceneIntent::OpenMenu { menu, anchor } = intent else {
                    return None;
                };
                Some((*menu, *anchor))
            }) {
                let _ = menu_bar.open(menu);
                self.scene = application_menu_scene(menu_bar, anchor, bounds);
                self.focus_return = ui.memory().focused();
            } else if context_requested {
                let anchor = ui
                    .input()
                    .pointer
                    .position
                    .map_or(Rect::new(0.0, 0.0, 1.0, 1.0), |point| {
                        Rect::new(point.x, point.y, 1.0, 1.0)
                    });
                self.scene = Some(context_menu_scene(actions, anchor, bounds));
                self.focus_return = ui.memory().focused();
            }
        }
        focus_return
    }
}

fn command_palette_requested(input: &UiInput) -> bool {
    input.keyboard.events.iter().any(|event| {
        event.state == KeyState::Pressed
            && !event.repeat
            && event.modifiers.ctrl
            && event.modifiers.shift
            && matches!(&event.key, Key::Character(value) if value.eq_ignore_ascii_case("p"))
    })
}

fn viewport_rect(bounds: Size) -> Rect {
    Rect::new(0.0, 0.0, bounds.width.max(0.0), bounds.height.max(0.0))
}

fn application_menu_scene(menu_bar: &MenuBar, anchor: Rect, bounds: Size) -> Option<OverlayScene> {
    let overlay = menu_bar.active_overlay(MenuBarOverlayRequest {
        overlay_id: APPLICATION_MENU_OVERLAY,
        kind: OverlayKind::Menu,
        anchor,
        size: Size::new(320.0, 128.0),
        placement: PopoverPlacement::Below,
        offset: 2.0,
        fit_viewport: true,
        viewport: viewport_rect(bounds),
        dismissal: OverlayDismissal::OutsideClickOrEscape,
        source: ActionSource::Menu,
        context: ActionContext::Editor,
    })?;
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::menu("Workspace commands", overlay));
    Some(scene)
}

fn context_menu_scene(actions: &DemoActionRegistry, anchor: Rect, bounds: Size) -> OverlayScene {
    let overlay = MenuOverlay::anchored(
        CONTEXT_MENU_OVERLAY,
        OverlayKind::ContextMenu,
        Menu::from_actions([actions.apply_shared_state().clone()]),
        anchor,
        Size::new(320.0, 40.0),
        PopoverPlacement::Below,
        2.0,
        true,
        viewport_rect(bounds),
        OverlayDismissal::OutsideClickOrEscape,
        ActionSource::Menu,
        ActionContext::Editor,
    );
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::menu("Viewport commands", overlay));
    scene
}

fn command_palette_scene(actions: &DemoActionRegistry, bounds: Size) -> OverlayScene {
    let viewport = viewport_rect(bounds);
    let anchor = Rect::new(viewport.width * 0.5, 24.0, 1.0, 1.0);
    let overlay = CommandPaletteOverlay::anchored_from_actions(
        COMMAND_PALETTE_OVERLAY,
        &[actions.apply_shared_state().clone()],
        anchor,
        Size::new(360.0, 96.0),
        PopoverPlacement::Below,
        4.0,
        true,
        viewport,
        OverlayDismissal::OutsideClickOrEscape,
        ActionContext::Editor,
    );
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::command_palette(
        "Shared command palette",
        overlay,
    ));
    scene
}

fn color_notice_scene(
    notice: DemoColorOverlayNotice,
    bounds: Size,
    owner: Option<WidgetId>,
) -> OverlayScene {
    let viewport = viewport_rect(bounds);
    let mut scene = OverlayScene::new();
    match notice {
        DemoColorOverlayNotice::SaveFailed => {
            let rect = Rect::new((viewport.width - 320.0) * 0.5, 96.0, 320.0, 44.0);
            let entry = OverlayEntry::new(COLOR_FAILURE_POPOVER, OverlayKind::Popover, rect)
                .dismiss_on(OverlayDismissal::OutsideClickOrEscape);
            scene.push(OverlaySceneSurface::passive(
                entry,
                "Color recovery hint",
                "Save failed without mutation. Dismiss and retry.",
            ));
        }
        DemoColorOverlayNotice::SaveRecovered => {
            let focus = owner.map_or_else(ModalFocusContainment::new, |owner| {
                ModalFocusContainment::new().with_return_focus(owner)
            });
            let dialog = ModalDialog::new(
                WidgetId::from_key("edit-workspace.color-recovery"),
                "Color style recovered",
            )
            .with_body("Explicit sRGB color and gradient serialization succeeded.")
            .with_focus(focus);
            let rect = Rect::new((viewport.width - 360.0) * 0.5, 128.0, 360.0, 96.0);
            scene.push(OverlaySceneSurface::modal(ModalDialogOverlay::placed(
                COLOR_RECOVERY_MODAL,
                rect,
                dialog,
                OverlayDismissal::OutsideClickOrEscape,
                ActionContext::Editor,
            )));
        }
    }
    scene
}
