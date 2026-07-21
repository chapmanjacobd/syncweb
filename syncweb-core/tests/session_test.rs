use anyhow::{Context, ensure};
use std::time::Duration;

use n0_future::StreamExt;
use syncweb_core::sync::{IntentHandle, SessionMode, SyncCommand, SyncEvent};

#[test]
fn test_reconcile_once() -> anyhow::Result<()> {
    let mode = SessionMode::ReconcileOnce;
    ensure!(!mode.is_continuous());
    Ok(())
}

#[test]
fn test_continuous() -> anyhow::Result<()> {
    let mode = SessionMode::Continuous;
    ensure!(mode.is_continuous());
    ensure!(!SessionMode::ReconcileOnce.is_continuous());
    Ok(())
}

#[tokio::test]
async fn test_intent_handle_stream() -> anyhow::Result<()> {
    let (events, _commands, mut handle) = IntentHandle::channel();

    events.send(SyncEvent::Started).context("send started")?;
    events
        .send(SyncEvent::Progress {
            completed: 50,
            total: Some(100),
        })
        .context("send progress")?;
    events.send(SyncEvent::Finished).context("send finished")?;

    let first = tokio::time::timeout(Duration::from_secs(2), handle.next())
        .await
        .context("timeout")?
        .context("stream ended")?;
    anyhow::ensure!(first == SyncEvent::Started);

    let second = tokio::time::timeout(Duration::from_secs(2), handle.next())
        .await
        .context("timeout")?
        .context("stream ended")?;
    anyhow::ensure!(
        second
            == SyncEvent::Progress {
                completed: 50,
                total: Some(100)
            },
        "expected progress event"
    );

    let third = tokio::time::timeout(Duration::from_secs(2), handle.next())
        .await
        .context("timeout")?
        .context("stream ended")?;
    anyhow::ensure!(third == SyncEvent::Finished);
    Ok(())
}

#[tokio::test]
async fn test_intent_handle_sink() -> anyhow::Result<()> {
    let (_events, mut commands, handle) = IntentHandle::channel();

    handle.pause().context("pause should succeed")?;
    handle.resume().context("resume should succeed")?;
    handle.cancel().context("cancel should succeed")?;

    let cmd = tokio::time::timeout(Duration::from_millis(100), commands.recv())
        .await
        .context("timeout")?
        .context("channel closed")?;
    anyhow::ensure!(cmd == SyncCommand::Pause);

    let cmd2 = tokio::time::timeout(Duration::from_millis(100), commands.recv())
        .await
        .context("timeout")?
        .context("channel closed")?;
    anyhow::ensure!(cmd2 == SyncCommand::Resume);

    let cmd3 = tokio::time::timeout(Duration::from_millis(100), commands.recv())
        .await
        .context("timeout")?
        .context("channel closed")?;
    anyhow::ensure!(cmd3 == SyncCommand::Cancel);
    Ok(())
}
