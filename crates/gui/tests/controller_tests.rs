use std::sync::Arc;
use std::time::Duration;

use miao_gui::controller::AppController;
use miao_gui::messages::{AppCommand, AppEvent};
use tokio::runtime::Runtime;

fn setup() -> (Arc<Runtime>, AppController) {
    let rt = Arc::new(Runtime::new().unwrap());
    let ctx = egui::Context::default();
    let controller = AppController::new(&rt, ctx);
    (rt, controller)
}

fn recv_event_blocking(rt: &Runtime, controller: &mut AppController, timeout_ms: u64) -> Option<AppEvent> {
    rt.block_on(async {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            if let Some(ev) = controller.try_recv() {
                return Some(ev);
            }
            if tokio::time::Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
}

fn drain_events_blocking(rt: &Runtime, controller: &mut AppController, timeout_ms: u64) -> Vec<AppEvent> {
    rt.block_on(async {
        let mut events = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            if let Some(ev) = controller.try_recv() {
                events.push(ev);
            }
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        events
    })
}

#[test]
fn version_manifest_fetch_produces_event() {
    let (rt, mut controller) = setup();

    controller.send(AppCommand::FetchVersionManifest {
        mirror: miao_core::config::DownloadMirror::Official,
    });

    let event = recv_event_blocking(&rt, &mut controller, 15000);
    assert!(event.is_some(), "Should receive VersionsFetched event");

    match event.unwrap() {
        AppEvent::VersionsFetched { all_versions, releases } => {
            assert!(
                !all_versions.is_empty() || releases.is_empty(),
                "Should have versions or gracefully return empty on network failure"
            );
        }
        other => panic!("Expected VersionsFetched, got {:?}", other),
    }
}

#[test]
fn loader_versions_fetch_produces_event() {
    let (rt, mut controller) = setup();

    controller.send(AppCommand::FetchLoaderVersions {
        mc_version: "1.21.4".to_string(),
    });

    let event = recv_event_blocking(&rt, &mut controller, 15000);
    assert!(event.is_some(), "Should receive loader event");

    match event.unwrap() {
        AppEvent::LoaderVersionsFetched { versions } => {
            assert!(!versions.is_empty(), "Should have at least one loader for 1.21.4");
        }
        AppEvent::LoaderFetchFailed => {}
        other => panic!("Expected LoaderVersionsFetched or LoaderFetchFailed, got {:?}", other),
    }
}

#[test]
fn check_for_updates_produces_event_or_nothing() {
    let (rt, mut controller) = setup();

    controller.send(AppCommand::CheckForUpdates);

    let events = drain_events_blocking(&rt, &mut controller, 10000);
    for ev in &events {
        match ev {
            AppEvent::UpdateAvailable(_) => {}
            other => panic!("Unexpected event from CheckForUpdates: {:?}", other),
        }
    }
}

#[test]
fn cancel_task_when_no_active_task_is_noop() {
    let (rt, mut controller) = setup();

    controller.send(AppCommand::CancelCurrentTask);

    rt.block_on(async { tokio::time::sleep(Duration::from_millis(200)).await });
    let event = controller.try_recv();
    assert!(event.is_none(), "Cancel with no active task should produce no event");
}

#[test]
fn mod_search_produces_results_or_error() {
    let (rt, mut controller) = setup();

    controller.send(AppCommand::SearchMods {
        query: "sodium".to_string(),
        mc_version: "1.21.4".to_string(),
        loader: Some("fabric".to_string()),
    });

    let event = recv_event_blocking(&rt, &mut controller, 15000);
    assert!(event.is_some(), "Should receive mod search event");

    match event.unwrap() {
        AppEvent::ModSearchResults(hits) => {
            assert!(!hits.is_empty(), "Sodium should have search results");
        }
        AppEvent::ModError(_) => {}
        other => panic!("Expected ModSearchResults or ModError, got {:?}", other),
    }
}

#[test]
fn create_instance_with_invalid_version_fails() {
    let (rt, mut controller) = setup();
    let tmp = tempfile::tempdir().unwrap();
    let config = miao_core::config::LauncherConfig {
        data_dir: tmp.path().to_path_buf(),
        ..Default::default()
    };

    controller.send(AppCommand::CreateInstance {
        ver: miao_core::version::VersionInfo {
            id: "99.99.99".to_string(),
            url: "https://invalid.example.com/not-real.json".to_string(),
            version_type: miao_core::version::VersionType::Release,
            release_time: String::new(),
        },
        name: "test-fail".to_string(),
        loader: None,
        config,
    });

    let events = drain_events_blocking(&rt, &mut controller, 15000);
    let has_failure = events.iter().any(|ev| matches!(ev, AppEvent::InstallFinished { success: false, .. }));
    assert!(has_failure, "Invalid version URL should produce InstallFinished with success=false");
}
