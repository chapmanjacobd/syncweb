use std::time::Duration;

use n0_future::StreamExt;
use syncweb_core::sync::{IntentHandle, SessionMode, SyncCommand, SyncEvent};

#[test]
fn test_reconcile_once() {
    let mode = SessionMode::ReconcileOnce;
    assert!(!mode.is_continuous());
}

#[test]
fn test_continuous() {
    let mode = SessionMode::Continuous;
    assert!(mode.is_continuous());
    assert!(!SessionMode::ReconcileOnce.is_continuous());
}

#[tokio::test]
async fn test_intent_handle_stream() {
    let (events, _commands, mut handle) = IntentHandle::channel();

    events.send(SyncEvent::Started).expect("send started");
    events
        .send(SyncEvent::Progress {
            completed: 50,
            total: Some(100),
        })
        .expect("send progress");
    events.send(SyncEvent::Finished).expect("send finished");

    let first = tokio::time::timeout(Duration::from_secs(2), handle.next())
        .await
        .expect("timeout")
        .expect("stream ended");
    assert_eq!(first, SyncEvent::Started);

    let second = tokio::time::timeout(Duration::from_secs(2), handle.next())
        .await
        .expect("timeout")
        .expect("stream ended");
    assert_eq!(
        second,
        SyncEvent::Progress {
            completed: 50,
            total: Some(100)
        }
    );

    let third = tokio::time::timeout(Duration::from_secs(2), handle.next())
        .await
        .expect("timeout")
        .expect("stream ended");
    assert_eq!(third, SyncEvent::Finished);
}

#[tokio::test]
async fn test_intent_handle_sink() {
    let (_events, mut commands, handle) = IntentHandle::channel();

    handle.pause().expect("pause should succeed");
    handle.resume().expect("resume should succeed");
    handle.cancel().expect("cancel should succeed");

    let cmd = tokio::time::timeout(Duration::from_millis(100), commands.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(cmd, SyncCommand::Pause);

    let cmd2 = tokio::time::timeout(Duration::from_millis(100), commands.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(cmd2, SyncCommand::Resume);

    let cmd3 = tokio::time::timeout(Duration::from_millis(100), commands.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(cmd3, SyncCommand::Cancel);
}
