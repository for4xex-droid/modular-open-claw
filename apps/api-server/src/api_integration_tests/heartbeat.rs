/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::api_integration_tests::common::create_test_server;
use crate::internal_services::heartbeat::check_and_notify_version;
use aiome_core::traits::SettingsOps;
use aiome_core_contracts::events::CoreEvent;

#[tokio::test]
async fn test_version_notification_on_first_startup() {
    let (_server, state, _tmp) = create_test_server().await;

    // 1. Subscribe to events
    let mut rx = state.event_sender.get_inner().subscribe();

    // 2. Run check_and_notify_version
    check_and_notify_version(&state)
        .await
        .expect("Failed to run check_and_notify_version");

    // 3. Verify notification is sent
    let event = rx.recv().await.expect("No event received");
    if let CoreEvent::ProactiveTalk {
        message,
        channel_id,
    } = event
    {
        assert!(message.contains("v1.0") || message.contains("リリース"));
        assert_eq!(channel_id, 0);
    } else {
        panic!("Expected CoreEvent::ProactiveTalk");
    }

    // 4. Verify setting is updated
    let last_notified = state
        .job_queue
        .get_setting_value("last_notified_version")
        .await
        .unwrap();
    assert!(last_notified.is_some());
    assert_eq!(last_notified.unwrap(), "v1.0.2");

    // 5. Run check_and_notify_version again and verify no second notification is sent
    // Clear any events
    while rx.try_recv().is_ok() {}

    check_and_notify_version(&state)
        .await
        .expect("Failed to run second check_and_notify_version");

    assert!(
        rx.try_recv().is_err(),
        "Duplicate notification sent on second run"
    );
}
