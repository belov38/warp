use ::local_control::protocol::{
    Action, ActionKind, PaneSelector, PaneTarget, SessionSelector, SessionTarget, TargetSelector,
    WindowSelector, WindowTarget,
};
use ::local_control::{ErrorCode, InstanceId};
use warpui::App;

use super::{pane_links_clear, pane_links_remove, pane_links_set, tab_links_set};
use crate::local_control::LocalControlBridge;
use crate::local_control::handlers::metadata::pane_list;
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
            assert_eq!(replaced["links"][1]["url"], "https://b2");

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
            assert_eq!(panes[0]["links"][0]["url"], "https://a");
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
