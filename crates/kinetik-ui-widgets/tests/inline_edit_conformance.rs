//! Inline edit and rename contract conformance tests.

use kinetik_ui_core::{
    ClipboardText, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, Rect, Size, UiInput,
    UiMemory, WidgetId, default_dark_theme,
};
use kinetik_ui_text::TextEditState;
use kinetik_ui_widgets::{
    AssetBrowserItem, AssetBrowserLayout, AssetBrowserModel, AssetBrowserViewMode, GridColumns,
    GridLayout, InlineEditCancelReason, InlineEditCommitReason, InlineEditDraftDisposition,
    InlineEditDraftPolicy, InlineEditDraftStatus, InlineEditFocusLossPolicy, InlineEditRequest,
    InlineEditSession, ItemId, ListLayout, OutlinerItem, OutlinerLayout, OutlinerModel,
    OutlinerRowFlags, Selection, TreeExpansion, asset_browser_item_widget_id,
    asset_browser_semantics, inline_edit_widget_id, outliner_row_widget_id, outliner_semantics,
    text_field,
};

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn key_input(key: Key) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                key,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    }
}

fn draft_policy() -> InlineEditDraftPolicy {
    InlineEditDraftPolicy::new(
        InlineEditDraftDisposition::Reject,
        InlineEditDraftDisposition::Cancel,
    )
}

#[test]
fn rename_starts_from_selected_outliner_item_and_preserves_selection() {
    let model = OutlinerModel::new(vec![
        OutlinerItem::new(id(10), "World"),
        OutlinerItem::new(id(20), "Camera"),
    ]);
    let mut selection = Selection::new();
    selection.replace(id(20));
    let root = WidgetId::from_key("outliner");

    let begin = model
        .inline_rename_begin_from_selection(&selection, root)
        .expect("selected item can rename");
    let session = InlineEditSession::new(begin, InlineEditFocusLossPolicy::Commit, draft_policy());

    assert_eq!(session.target, id(20));
    assert_eq!(session.initial_text, "Camera");
    assert_eq!(session.text_widget_id, inline_edit_widget_id(root, id(20)));
    assert!(session.target_selected(&selection));
    assert_eq!(selection.selected(), vec![id(20)]);
    assert_eq!(selection.active, Some(id(20)));
}

#[test]
fn inline_rename_requires_active_item_to_remain_selected() {
    let outliner = OutlinerModel::new(vec![
        OutlinerItem::new(id(10), "World"),
        OutlinerItem::new(id(20), "Camera"),
    ]);
    let assets = AssetBrowserModel::new(vec![
        AssetBrowserItem::new(id(10), "World", "scene"),
        AssetBrowserItem::new(id(20), "Camera", "image"),
    ]);
    let mut selection = Selection::new();
    selection.replace(id(10));
    selection.toggle(id(20));
    selection.toggle(id(20));

    assert_eq!(selection.selected(), vec![id(10)]);
    assert_eq!(selection.active, Some(id(20)));
    assert!(!selection.contains(id(20)));
    assert!(
        outliner
            .inline_rename_begin_from_selection(&selection, WidgetId::from_key("outliner"))
            .is_none()
    );
    assert!(
        assets
            .inline_rename_begin_from_selection(&selection, WidgetId::from_key("assets"))
            .is_none()
    );
}

#[test]
fn text_ownership_targets_inline_edit_widget_id_and_clipboard_is_isolated() {
    let model = AssetBrowserModel::new(vec![
        AssetBrowserItem::new(id(10), "Image", "image"),
        AssetBrowserItem::new(id(20), "Material", "material"),
    ]);
    let mut selection = Selection::new();
    selection.replace(id(20));
    let root = WidgetId::from_key("assets");
    let begin = model
        .inline_rename_begin_from_selection(&selection, root)
        .expect("selected asset can rename");
    let session = InlineEditSession::new(begin, InlineEditFocusLossPolicy::Cancel, draft_policy());

    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    memory.focus(session.text_widget_id);
    memory.set_text_input_owner(session.text_widget_id);
    let mut text = TextEditState::new(session.initial_text.clone());
    text.set_caret(text.text.len());
    let input = UiInput {
        clipboard_text: vec![
            ClipboardText::new(WidgetId::from_key("other-inline-edit"), " wrong"),
            ClipboardText::new(session.text_widget_id, " pasted"),
        ],
        ..UiInput::default()
    };

    let output = text_field(
        session.text_widget_id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut text,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(output.changed);
    assert_eq!(text.text, "Material pasted");
    assert_eq!(memory.text_input_owner(), Some(session.text_widget_id));
}

#[test]
fn draft_edit_commit_cancel_and_focus_loss_requests_are_deterministic() {
    let root = WidgetId::from_key("assets");
    let begin = AssetBrowserItem::new(id(7), "Old", "image")
        .inline_rename_begin_request(root)
        .expect("begin request");
    let mut session =
        InlineEditSession::new(begin, InlineEditFocusLossPolicy::Commit, draft_policy());

    let draft = session.set_draft("New");
    assert_eq!(draft.target, id(7));
    assert_eq!(draft.draft_text, "New");
    assert_eq!(draft.text_widget_id, inline_edit_widget_id(root, id(7)));

    let enter = session
        .keyboard_request(&key_input(Key::Enter))
        .expect("enter commits changed draft");
    assert!(matches!(
        enter,
        InlineEditRequest::Commit(ref request)
            if request.target == id(7)
                && request.draft_text == "New"
                && request.reason == InlineEditCommitReason::Enter
    ));

    let escape = session
        .keyboard_request(&key_input(Key::Escape))
        .expect("escape cancels");
    assert!(matches!(
        escape,
        InlineEditRequest::Cancel(ref request)
            if request.target == id(7)
                && request.draft_text == "New"
                && request.reason == InlineEditCancelReason::Escape
    ));

    let focus_loss = session.focus_loss_request().expect("focus loss commits");
    assert!(matches!(
        focus_loss,
        InlineEditRequest::Commit(ref request)
            if request.reason == InlineEditCommitReason::FocusLost
                && request.draft_text == "New"
    ));

    let cancel_focus = InlineEditSession::new(
        AssetBrowserItem::new(id(8), "Cancel me", "scene")
            .inline_rename_begin_request(root)
            .expect("begin request"),
        InlineEditFocusLossPolicy::Cancel,
        draft_policy(),
    );
    assert!(matches!(
        cancel_focus.focus_loss_request(),
        Some(InlineEditRequest::Cancel(ref request))
            if request.reason == InlineEditCancelReason::FocusLost
    ));

    let keep_open = InlineEditSession::new(
        AssetBrowserItem::new(id(9), "Keep me", "scene")
            .inline_rename_begin_request(root)
            .expect("begin request"),
        InlineEditFocusLossPolicy::KeepEditing,
        draft_policy(),
    );
    assert_eq!(keep_open.focus_loss_request(), None);
}

#[test]
fn empty_and_unchanged_draft_policy_is_explicit() {
    let root = WidgetId::from_key("outliner");
    let begin = OutlinerItem::new(id(30), "Camera")
        .inline_rename_begin_request(root)
        .expect("begin request");
    let mut session =
        InlineEditSession::new(begin, InlineEditFocusLossPolicy::Commit, draft_policy());

    session.set_draft("   ");
    let empty = session.resolve_commit(InlineEditCommitReason::Enter);
    assert_eq!(empty.draft_status, InlineEditDraftStatus::Empty);
    assert_eq!(empty.disposition, InlineEditDraftDisposition::Reject);
    assert_eq!(empty.request, None);

    session.set_draft("Camera");
    let unchanged = session.resolve_commit(InlineEditCommitReason::Enter);
    assert_eq!(unchanged.draft_status, InlineEditDraftStatus::Unchanged);
    assert_eq!(unchanged.disposition, InlineEditDraftDisposition::Cancel);
    assert!(matches!(
        unchanged.request,
        Some(InlineEditRequest::Cancel(ref request))
            if request.reason == InlineEditCancelReason::DraftPolicy
    ));

    session.set_draft("Camera A");
    let changed = session.resolve_commit(InlineEditCommitReason::Enter);
    assert_eq!(changed.draft_status, InlineEditDraftStatus::Changed);
    assert_eq!(changed.disposition, InlineEditDraftDisposition::Commit);
    assert!(matches!(
        changed.request,
        Some(InlineEditRequest::Commit(ref request))
            if request.target == id(30) && request.draft_text == "Camera A"
    ));
}

#[test]
fn disabled_read_only_and_non_renamable_items_suppress_begin_requests() {
    let root = WidgetId::from_key("rename-root");

    let mut disabled = OutlinerRowFlags::new();
    disabled.disabled = true;
    let mut read_only = OutlinerRowFlags::new();
    read_only.read_only = true;
    let mut non_renamable = OutlinerRowFlags::new();
    non_renamable.renamable = false;
    let model = OutlinerModel::new(vec![
        OutlinerItem::new(id(1), "Disabled").with_flags(disabled),
        OutlinerItem::new(id(2), "Read only").with_flags(read_only),
        OutlinerItem::new(id(3), "Fixed").with_flags(non_renamable),
        OutlinerItem::new(id(4), "Editable"),
    ]);
    let rows = model.visible_rows(&TreeExpansion::new());

    assert!(rows[0].inline_rename_begin_request(root).is_none());
    assert!(rows[1].inline_rename_begin_request(root).is_none());
    assert!(rows[2].inline_rename_begin_request(root).is_none());
    assert!(rows[3].inline_rename_begin_request(root).is_some());

    assert!(
        AssetBrowserItem::new(id(10), "Disabled", "image")
            .disabled(true)
            .inline_rename_begin_request(root)
            .is_none()
    );
    assert!(
        AssetBrowserItem::new(id(11), "Read only", "image")
            .read_only(true)
            .inline_rename_begin_request(root)
            .is_none()
    );
    assert!(
        AssetBrowserItem::new(id(12), "Fixed", "image")
            .renamable(false)
            .inline_rename_begin_request(root)
            .is_none()
    );
}

#[test]
fn outliner_and_asset_semantics_expose_rename_only_when_available() {
    let root = WidgetId::from_key("outliner");
    let mut fixed = OutlinerRowFlags::new();
    fixed.renamable = false;
    let rows = OutlinerModel::new(vec![
        OutlinerItem::new(id(1), "Editable"),
        OutlinerItem::new(id(2), "Fixed").with_flags(fixed),
    ])
    .visible_rows(&TreeExpansion::new());
    let zones = OutlinerLayout::new(20.0, 12.0).visible_row_zones(
        Rect::new(0.0, 0.0, 200.0, 80.0),
        &rows,
        0.0,
        0,
    );
    let outliner_nodes = outliner_semantics(
        root,
        Rect::new(0.0, 0.0, 200.0, 80.0),
        &zones,
        &Selection::new(),
        "Scene",
    );
    let editable = outliner_nodes
        .iter()
        .find(|node| node.id == outliner_row_widget_id(root, id(1)))
        .expect("editable row");
    let fixed = outliner_nodes
        .iter()
        .find(|node| node.id == outliner_row_widget_id(root, id(2)))
        .expect("fixed row");
    assert!(
        editable
            .actions
            .iter()
            .any(|action| action.label == "Rename row")
    );
    assert!(
        fixed
            .actions
            .iter()
            .all(|action| action.label != "Rename row")
    );

    let asset_root = WidgetId::from_key("assets");
    let assets = AssetBrowserModel::new(vec![
        AssetBrowserItem::new(id(10), "Editable", "image"),
        AssetBrowserItem::new(id(20), "Read only", "image").read_only(true),
    ]);
    let result = AssetBrowserLayout::new(
        AssetBrowserViewMode::List,
        GridLayout {
            columns: GridColumns::Fixed(1),
            item_size: Size::new(64.0, 64.0),
            gap: 4.0,
        },
        ListLayout::new(24.0),
    )
    .resolve(
        Rect::new(0.0, 0.0, 200.0, 60.0),
        &assets,
        0.0,
        &Selection::new(),
        None,
    );
    let asset_nodes = asset_browser_semantics(
        asset_root,
        Rect::new(0.0, 0.0, 200.0, 60.0),
        &result,
        "Assets",
    );
    let editable_asset = asset_nodes
        .iter()
        .find(|node| node.id == asset_browser_item_widget_id(asset_root, id(10)))
        .expect("editable asset");
    let read_only_asset = asset_nodes
        .iter()
        .find(|node| node.id == asset_browser_item_widget_id(asset_root, id(20)))
        .expect("read only asset");
    assert!(
        editable_asset
            .actions
            .iter()
            .any(|action| action.label == "Rename asset")
    );
    assert!(
        read_only_asset
            .actions
            .iter()
            .all(|action| action.label != "Rename asset")
    );
}
