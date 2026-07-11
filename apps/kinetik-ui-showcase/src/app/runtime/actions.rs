use super::super::{
    ACTION_COMPONENTS_RUN, ACTION_EDITOR_DOCK_JOIN, ACTION_EDITOR_DOCK_SWAP,
    ACTION_SYSTEMS_DISPATCH, ACTION_WORKSPACE_SAVE, ActionContext, ActionInvocation, ActionQueue,
    ActionRoutingContext, ActionSource, EditorShowcase, PlatformRequest, ShowcaseApp,
    ShowcaseWorkspaceSnapshot, showcase_action_router,
};
use crate::editor::{ACTION_DOCS, DOCUMENTATION_URL};

impl ShowcaseApp {
    pub(in crate::app) fn invoke_action(&mut self, id: &str, source: ActionSource) -> bool {
        let handled = self.editor.apply_action(id) || self.apply_showcase_action(id);
        self.finish_action_invocation(id, source, handled)
    }

    pub(in crate::app) fn apply_showcase_action(&mut self, action_id: &str) -> bool {
        match action_id {
            ACTION_COMPONENTS_RUN => {
                self.component_action_count += 1;
                self.status = format!("Component demo counter: {}", self.component_action_count);
                true
            }
            ACTION_SYSTEMS_DISPATCH => {
                self.systems_dispatch_count += 1;
                self.status = format!("Systems dispatches: {}", self.systems_dispatch_count);
                true
            }
            ACTION_WORKSPACE_SAVE => {
                self.workspace_snapshot = Some(self.capture_workspace_snapshot());
                "Workspace snapshot captured in memory".clone_into(&mut self.status);
                true
            }
            _ => false,
        }
    }

    pub(in crate::app) fn capture_workspace_snapshot(&self) -> ShowcaseWorkspaceSnapshot {
        ShowcaseWorkspaceSnapshot {
            page: self.page,
            selected_row: self.selected_row,
            selected_tab: self.selected_tab,
            checkbox: self.checkbox,
            toggle: self.toggle,
            radio: self.radio,
            strength: self.strength,
            dock_ratio: self.dock_ratio,
            dock_split_demo: self.dock_split_demo,
            zoom: self.zoom,
            stress: self.stress,
            name: self.name.text.clone(),
            number: self.number.text.clone(),
            search: self.search.text.clone(),
            notes: self.notes.text.clone(),
        }
    }

    pub(in crate::app) fn handle_applied_action_invocation(
        &mut self,
        invocation: &ActionInvocation,
    ) -> bool {
        self.finish_action_invocation(
            invocation.action_id.as_str(),
            invocation.source,
            Self::can_handle_action_id(invocation.action_id.as_str()),
        )
    }

    pub(in crate::app) fn finish_action_invocation(
        &mut self,
        action_id: &str,
        source: ActionSource,
        handled: bool,
    ) -> bool {
        if handled {
            if action_id == ACTION_DOCS {
                self.pending_platform_requests
                    .push(PlatformRequest::OpenUrl(DOCUMENTATION_URL.to_owned()));
            }
            self.record_action(action_id, source);
            true
        } else {
            self.ignore_action(action_id, source);
            false
        }
    }

    pub(in crate::app) fn can_handle_action_id(action_id: &str) -> bool {
        let mut editor = EditorShowcase::new();
        editor.apply_action(action_id)
            || Self::is_showcase_action(action_id)
            || Self::is_editor_rendered_action(action_id)
    }

    pub(in crate::app) fn handle_action_invocation(
        &mut self,
        invocation: &ActionInvocation,
    ) -> bool {
        if invocation.context == ActionContext::Editor {
            self.handle_applied_action_invocation(invocation)
        } else {
            self.invoke_action(invocation.action_id.as_str(), invocation.source)
        }
    }

    pub(in crate::app) fn handle_action_queue(
        &mut self,
        queue: &mut ActionQueue,
    ) -> Vec<ActionInvocation> {
        let invocations = queue.drain().collect::<Vec<_>>();
        for invocation in &invocations {
            self.handle_action_invocation(invocation);
        }
        invocations
    }

    pub(in crate::app) fn record_action(&mut self, _action_id: &str, _source: ActionSource) {
        self.action_count += 1;
    }

    pub(in crate::app) fn ignore_action(&mut self, action_id: &str, source: ActionSource) {
        self.status = format!("Ignored unhandled action {action_id} via {source:?}");
    }

    pub(in crate::app) fn is_showcase_action(action_id: &str) -> bool {
        matches!(
            action_id,
            ACTION_COMPONENTS_RUN | ACTION_SYSTEMS_DISPATCH | ACTION_WORKSPACE_SAVE
        )
    }

    pub(in crate::app) fn is_editor_rendered_action(action_id: &str) -> bool {
        matches!(action_id, ACTION_EDITOR_DOCK_JOIN | ACTION_EDITOR_DOCK_SWAP)
    }

    pub(in crate::app) fn resolve_shortcuts(&mut self, keyboard: &kinetik_ui::core::KeyboardInput) {
        let Some(invocation) = showcase_action_router(!self.editor.is_running())
            .resolve_shortcut_in_context(keyboard, self.action_context())
        else {
            return;
        };
        self.invoke_action(invocation.action_id.as_str(), invocation.source);
    }

    pub(in crate::app) fn action_context(&self) -> ActionRoutingContext {
        let Some(focused) = self.memory.focused() else {
            return ActionRoutingContext::new();
        };
        if self.memory.text_input_owner() == Some(focused) {
            ActionRoutingContext::new().with_text_input(focused)
        } else {
            ActionRoutingContext::new().with_focused_widget(focused)
        }
    }
}
