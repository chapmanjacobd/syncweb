use std::sync::Arc;

use anyhow::{Result, ensure};
use syncweb_core::{
    daemon::{DaemonHandle, DaemonState, DaemonStatus, IpcCommand, IpcRequest, IpcResponse, IpcServer, ManagedPool},
    folder::{FolderManager, SyncMode},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};

use crate::test_utils::TestDirectory;

async fn node(directory: &TestDirectory) -> Result<Arc<IrohNode>> {
    let root = directory.path().join("node");
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(Arc::new(
        IrohNode::new(
            identity,
            root.join("data"),
            RelayMode::Default,
            crate::test_utils::empty_member_keys(),
        )
        .await?,
    ))
}

#[tokio::test]
async fn daemon_enrich_sort_returns_path_peer_map() -> Result<()> {
    let directory = TestDirectory::new("syncweb-sort-enrich")?;
    let node = node(&directory).await?;
    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;
    let ns = folder.namespace_id();

    folder.set_blob("a.txt", b"content_a").await?;
    folder.set_blob("b.txt", b"content_b").await?;
    folder.set_blob("sub/c.txt", b"content_c").await?;

    let handle = DaemonHandle::new(DaemonState::new(
        std::process::id(),
        "node",
        1,
        directory.path(),
        DaemonStatus::Running,
    ));
    let pool = Arc::new(ManagedPool::new("sort-enrich-test", 1)?);
    let server = IpcServer::with_archive_context(directory.path().join("daemon.sock"), handle, node.clone(), pool);

    let response = server
        .handle_request(IpcRequest::new(IpcCommand::EnrichSort {
            path: std::path::PathBuf::from(ns.to_string()),
        }))
        .await;

    let data = unpack_enrich_data(response)?;
    ensure!(data.contains_key("a.txt"), "should contain a.txt: {data:?}");
    ensure!(data.contains_key("b.txt"), "should contain b.txt: {data:?}");
    ensure!(data.contains_key("sub/c.txt"), "should contain sub/c.txt: {data:?}");
    ensure!(data.len() == 3, "should have exactly 3 entries, got {}", data.len());
    for (path, count) in &data {
        ensure!(
            *count == 0,
            "peer count for {path} should be 0 (no resilience service): {count}"
        );
    }

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn daemon_enrich_sort_returns_empty_for_unknown_folder() -> Result<()> {
    let directory = TestDirectory::new("syncweb-sort-enrich-unknown")?;
    let node = node(&directory).await?;
    let handle = DaemonHandle::new(DaemonState::new(
        std::process::id(),
        "node",
        1,
        directory.path(),
        DaemonStatus::Running,
    ));
    let pool = Arc::new(ManagedPool::new("sort-enrich-unknown", 1)?);
    let server = IpcServer::with_archive_context(directory.path().join("daemon.sock"), handle, node.clone(), pool);

    let response = server
        .handle_request(IpcRequest::new(IpcCommand::EnrichSort {
            path: std::path::PathBuf::from("/nonexistent/path"),
        }))
        .await;

    let data = unpack_enrich_data(response)?;
    ensure!(data.is_empty(), "should be empty for unknown folder: {data:?}");

    node.stop().await?;
    Ok(())
}

fn unpack_enrich_data(response: IpcResponse) -> Result<std::collections::HashMap<String, usize>> {
    match response {
        IpcResponse::EnrichData(data) => Ok(data),
        IpcResponse::Ok { .. }
        | IpcResponse::Status(_)
        | IpcResponse::FolderList(_)
        | IpcResponse::DownloadComplete { .. }
        | IpcResponse::ImportFilesComplete { .. }
        | IpcResponse::ImportComplete(_)
        | IpcResponse::ExportComplete(_)
        | IpcResponse::Error { .. }
        | _ => {
            anyhow::bail!("expected EnrichData, got: {response:?}")
        }
    }
}
