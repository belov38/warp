use ::local_control::protocol::{
    Action, ActionKind, PaneSelector, PaneTarget, SessionSelector, SessionTarget, TargetSelector,
    WindowSelector, WindowTarget,
};
use ::local_control::{ErrorCode, InstanceId};
use warp_core::SessionId;
use warpui::{App, ViewHandle};

use super::{
    pane_links_clear, pane_links_remove, pane_links_set, tab_links_clear, tab_links_remove,
    tab_links_set,
};
use crate::local_control::LocalControlBridge;
use crate::local_control::handlers::metadata::pane_list;
use crate::pane_group::{Direction, PaneId};
use crate::workspace::Workspace;
use crate::workspace::view::tests::{initialize_app, mock_workspace};

fn set_action(label: &str, url: &str) -> Action {
    Action::with_params(
        ActionKind::PaneLinksSet,
        serde_json::json!({ "label": label, "url": url }),
    )
    .expect("params serialize")
}

fn remove_action(label: &str) -> Action {
    Action::with_params(
        ActionKind::PaneLinksRemove,
        serde_json::json!({ "label": label }),
    )
    .expect("params serialize")
}

/// The test platform never reports an active window, so every target is scoped
/// to the window `mock_workspace` created.
fn window_scoped_target(app: &App) -> TargetSelector {
    let window_ids = app.window_ids();
    let [window_id] = window_ids.as_slice() else {
        panic!(
            "expected exactly one mock window, found {}",
            window_ids.len()
        );
    };
    TargetSelector {
        window: Some(WindowTarget::Id {
            id: WindowSelector(window_id.to_string()),
        }),
        ..TargetSelector::default()
    }
}

fn labels(value: &serde_json::Value) -> Vec<String> {
    value["links"]
        .as_array()
        .expect("links array")
        .iter()
        .map(|link| link["label"].as_str().expect("label").to_owned())
        .collect()
}

#[test]
fn pane_links_set_appends_replaces_and_caps() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _workspace = mock_workspace(&mut app);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = window_scoped_target(&app);

        bridge.update(&mut app, |_, ctx| {
            let first = pane_links_set(&instance_id, &target, &set_action("a", "https://a"), ctx)
                .expect("first set");
            assert_eq!(first["action"], "pane.links.set");
            assert_eq!(first["ok"], true);
            assert_eq!(labels(&first), ["a"]);

            pane_links_set(&instance_id, &target, &set_action("b", "https://b"), ctx)
                .expect("second set");
            let third = pane_links_set(&instance_id, &target, &set_action("c", "https://c"), ctx)
                .expect("third set");
            assert_eq!(labels(&third), ["a", "b", "c"]);

            let replaced =
                pane_links_set(&instance_id, &target, &set_action("b", "https://b2"), ctx)
                    .expect("replace keeps position");
            assert_eq!(labels(&replaced), ["a", "b", "c"]);
            assert_eq!(replaced["links"][1]["url"], "https://b2/");

            let err = pane_links_set(&instance_id, &target, &set_action("d", "https://d"), ctx)
                .expect_err("fourth link is rejected");
            assert_eq!(err.code, ErrorCode::InvalidParams);
            assert!(
                err.message.contains("already has 3 links"),
                "{}",
                err.message
            );
        });
    });
}

#[test]
fn pane_links_set_rejects_bad_label_and_url_without_changing_state() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _workspace = mock_workspace(&mut app);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = window_scoped_target(&app);

        bridge.update(&mut app, |_, ctx| {
            let err = pane_links_set(&instance_id, &target, &set_action("  ", "https://a"), ctx)
                .expect_err("empty label");
            assert_eq!(err.code, ErrorCode::InvalidParams);

            let err = pane_links_set(
                &instance_id,
                &target,
                &set_action("x", "javascript:alert(1)"),
                ctx,
            )
            .expect_err("bad scheme");
            assert_eq!(err.code, ErrorCode::InvalidParams);
            assert!(err.message.contains("javascript"), "{}", err.message);

            let cleared = pane_links_clear(&instance_id, &target, ctx).expect("clear");
            assert!(labels(&cleared).is_empty());
        });
    });
}

#[test]
fn pane_links_remove_and_clear() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _workspace = mock_workspace(&mut app);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = window_scoped_target(&app);

        bridge.update(&mut app, |_, ctx| {
            pane_links_set(&instance_id, &target, &set_action("a", "https://a"), ctx).unwrap();
            pane_links_set(&instance_id, &target, &set_action("b", "https://b"), ctx).unwrap();

            let removed = pane_links_remove(&instance_id, &target, &remove_action("a"), ctx)
                .expect("remove a");
            assert_eq!(labels(&removed), ["b"]);

            let err = pane_links_remove(&instance_id, &target, &remove_action("zzz"), ctx)
                .expect_err("unknown label");
            assert_eq!(err.code, ErrorCode::InvalidParams);
            assert!(
                err.message.contains("no link with label"),
                "{}",
                err.message
            );

            let cleared = pane_links_clear(&instance_id, &target, ctx).expect("clear");
            assert!(labels(&cleared).is_empty());
            let again = pane_links_clear(&instance_id, &target, ctx).expect("clear is idempotent");
            assert!(labels(&again).is_empty());
        });
    });
}

#[test]
fn pane_list_reports_links() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _workspace = mock_workspace(&mut app);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = window_scoped_target(&app);

        bridge.update(&mut app, |_, ctx| {
            pane_links_set(&instance_id, &target, &set_action("a", "https://a"), ctx).unwrap();
            let listed = pane_list(&target, ctx).expect("pane.list");
            let panes = listed["panes"].as_array().expect("panes");
            assert_eq!(panes.len(), 1);
            assert_eq!(panes[0]["links"][0]["label"], "a");
            assert_eq!(panes[0]["links"][0]["url"], "https://a/");
        });
    });
}

#[test]
fn tab_links_set_targets_the_focused_pane() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _workspace = mock_workspace(&mut app);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = window_scoped_target(&app);
        let action = Action::with_params(
            ActionKind::TabLinksSet,
            serde_json::json!({ "label": "t", "url": "https://t" }),
        )
        .unwrap();

        bridge.update(&mut app, |_, ctx| {
            let result = tab_links_set(&instance_id, &target, &action, ctx).expect("tab set");
            assert_eq!(result["action"], "tab.links.set");
            assert!(result["pane_id"].is_string());
            assert_eq!(labels(&result), ["t"]);
            // The same pane is visible through the pane-scoped read.
            let listed = pane_list(&target, ctx).expect("pane.list");
            assert_eq!(listed["panes"][0]["links"][0]["label"], "t");
        });
    });
}

#[test]
fn session_selector_cannot_be_combined_with_pane_or_tab_selectors() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _workspace = mock_workspace(&mut app);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = TargetSelector {
            pane: Some(PaneTarget::Id {
                id: PaneSelector("Pane Terminal (1)".to_owned()),
            }),
            session: Some(SessionTarget::Id {
                id: SessionSelector("12345".to_owned()),
            }),
            ..window_scoped_target(&app)
        };

        bridge.update(&mut app, |_, ctx| {
            let err = pane_links_set(&instance_id, &target, &set_action("a", "https://a"), ctx)
                .expect_err("conflicting selectors");
            assert_eq!(err.code, ErrorCode::InvalidParams);
        });
    });
}

#[test]
fn unknown_numeric_session_is_missing_target() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _workspace = mock_workspace(&mut app);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = TargetSelector {
            session: Some(SessionTarget::Id {
                id: SessionSelector("987654321".to_owned()),
            }),
            ..window_scoped_target(&app)
        };

        bridge.update(&mut app, |_, ctx| {
            let err = pane_links_set(&instance_id, &target, &set_action("a", "https://a"), ctx)
                .expect_err("no pane has that session");
            assert_eq!(err.code, ErrorCode::MissingTarget);
        });
    });
}

#[test]
fn active_session_selector_targets_the_active_pane() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _workspace = mock_workspace(&mut app);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = TargetSelector {
            session: Some(SessionTarget::Active),
            ..window_scoped_target(&app)
        };

        bridge.update(&mut app, |_, ctx| {
            let result = pane_links_set(&instance_id, &target, &set_action("s", "https://s"), ctx)
                .expect("active session resolves");
            assert_eq!(labels(&result), ["s"]);
        });
    });
}

/// Splits the mock workspace's only tab into two terminal panes, gives each a
/// distinct shell session id, focuses the second, and returns
/// (first_pane_id, second_pane_id).
fn split_tab_with_sessions(app: &mut App, workspace: &ViewHandle<Workspace>) -> (PaneId, PaneId) {
    let pane_group = workspace.read(app, |workspace, _| {
        workspace.active_tab_pane_group().clone()
    });
    let first = pane_group.read(app, |pane_group, ctx| pane_group.focused_pane_id(ctx));
    let second = pane_group.update(app, |pane_group, ctx| {
        PaneId::from(pane_group.add_terminal_pane(Direction::Right, None, ctx))
    });
    for (pane_id, session) in [(first, 1001_u64), (second, 1002_u64)] {
        let terminal = pane_group
            .read(app, |pane_group, ctx| {
                pane_group.terminal_view_from_pane_id(pane_id, ctx)
            })
            .expect("terminal pane");
        terminal.update(app, |view, _| {
            view.set_active_block_session_id_for_tests(SessionId::from(session));
        });
    }
    let focused = pane_group.read(app, |pane_group, ctx| pane_group.focused_pane_id(ctx));
    assert_eq!(focused, second, "the new split pane should be focused");
    (first, second)
}

fn session_target(app: &App, id: &str) -> TargetSelector {
    TargetSelector {
        session: Some(SessionTarget::Id {
            id: SessionSelector(id.to_owned()),
        }),
        ..window_scoped_target(app)
    }
}

#[test]
fn numeric_session_resolves_the_unfocused_pane() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);
        let (first, second) = split_tab_with_sessions(&mut app, &workspace);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = session_target(&app, "1001");
        let all_panes_target = window_scoped_target(&app);

        bridge.update(&mut app, |_, ctx| {
            let result = pane_links_set(&instance_id, &target, &set_action("a", "https://a"), ctx)
                .expect("numeric session resolves");
            assert_eq!(result["pane_id"], first.to_string());
            assert_ne!(result["pane_id"], second.to_string());

            let listed = pane_list(&all_panes_target, ctx).expect("pane.list");
            let panes = listed["panes"].as_array().expect("panes");
            let with_links: Vec<_> = panes
                .iter()
                .filter(|pane| !pane["links"].as_array().expect("links").is_empty())
                .collect();
            assert_eq!(with_links.len(), 1);
            assert_eq!(with_links[0]["pane_id"], first.to_string());
        });
    });
}

#[test]
fn tab_links_with_session_targets_the_session_pane_not_the_focused_one() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);
        let (first, _second) = split_tab_with_sessions(&mut app, &workspace);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = session_target(&app, "1001");
        let action = Action::with_params(
            ActionKind::TabLinksSet,
            serde_json::json!({ "label": "t", "url": "https://t/" }),
        )
        .unwrap();

        bridge.update(&mut app, |_, ctx| {
            let result =
                tab_links_set(&instance_id, &target, &action, ctx).expect("tab set via session");
            assert_eq!(result["pane_id"], first.to_string());
        });
    });
}

#[test]
fn string_form_session_id_resolves_by_pane_id() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);
        let (first, _second) = split_tab_with_sessions(&mut app, &workspace);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = session_target(&app, &format!("  {first}  "));

        bridge.update(&mut app, |_, ctx| {
            let result = pane_links_set(&instance_id, &target, &set_action("s", "https://s"), ctx)
                .expect("Pane Terminal (N) form resolves, trimmed");
            assert_eq!(result["pane_id"], first.to_string());
        });
    });
}

#[test]
fn duplicate_session_ids_are_ambiguous() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);
        let (first, second) = split_tab_with_sessions(&mut app, &workspace);
        let pane_group = workspace.read(&app, |workspace, _| {
            workspace.active_tab_pane_group().clone()
        });
        for pane_id in [first, second] {
            let terminal = pane_group
                .read(&app, |pane_group, ctx| {
                    pane_group.terminal_view_from_pane_id(pane_id, ctx)
                })
                .expect("terminal pane");
            terminal.update(&mut app, |view, _| {
                view.set_active_block_session_id_for_tests(SessionId::from(7_u64));
            });
        }
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = session_target(&app, "7");

        bridge.update(&mut app, |_, ctx| {
            let err = pane_links_set(&instance_id, &target, &set_action("a", "https://a"), ctx)
                .expect_err("two panes share the session id");
            assert_eq!(err.code, ErrorCode::AmbiguousTarget);
        });
    });
}

#[test]
fn tab_links_remove_and_clear_target_the_focused_pane() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _workspace = mock_workspace(&mut app);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = Some(InstanceId("inst_test".to_owned()));
        let target = window_scoped_target(&app);
        let set = |label: &str| {
            Action::with_params(
                ActionKind::TabLinksSet,
                serde_json::json!({ "label": label, "url": format!("https://{label}/") }),
            )
            .unwrap()
        };
        let remove = Action::with_params(
            ActionKind::TabLinksRemove,
            serde_json::json!({ "label": "a" }),
        )
        .unwrap();

        bridge.update(&mut app, |_, ctx| {
            tab_links_set(&instance_id, &target, &set("a"), ctx).unwrap();
            tab_links_set(&instance_id, &target, &set("b"), ctx).unwrap();
            let removed = tab_links_remove(&instance_id, &target, &remove, ctx).expect("remove");
            assert_eq!(labels(&removed), ["b"]);
            assert_eq!(removed["action"], "tab.links.remove");
            let cleared = tab_links_clear(&instance_id, &target, ctx).expect("clear");
            assert!(labels(&cleared).is_empty());
            assert_eq!(cleared["action"], "tab.links.clear");
        });
    });
}
