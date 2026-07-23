use std::{fs, sync::Arc};

use anyhow::{Result, ensure};
use syncweb_core::{
    daemon::{DaemonHandle, DaemonState, DaemonStatus, IpcCommand, IpcRequest, IpcResponse, IpcServer, ManagedPool},
    folder::{CollectionEntry, CollectionManifest, CollectionStore, FolderManager, SyncMode},
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
        IrohNode::new(identity, root.join("data"), RelayMode::Default).await?,
    ))
}

#[tokio::test]
async fn daemon_ipc_archive_operations_use_shared_pool() -> Result<()> {
    let directory = TestDirectory::new("syncweb-daemon-archive-test")?;
    let node = node(&directory).await?;
    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;
    let content = b"daemon archive";
    let content_hash = node.blob_store().add_bytes(content).await?;
    let mut manifest = CollectionManifest::new(uuid::Uuid::new_v4(), "1.0.0");
    manifest.entries.push(CollectionEntry::new(
        content_hash,
        "daemon.txt",
        u64::try_from(content.len())?,
    )?);
    CollectionStore::new(
        folder.doc().clone(),
        folder.author(),
        node.blob_store().clone(),
        node.docs_engine().clone(),
    )
    .publish(&manifest, 1)
    .await?;

    let handle = DaemonHandle::new(DaemonState::new(
        std::process::id(),
        "node",
        1,
        directory.path(),
        DaemonStatus::Running,
    ));
    let pool = Arc::new(ManagedPool::new("daemon-archive-test", 1)?);
    let server = IpcServer::with_archive_context(directory.path().join("daemon.sock"), handle, node.clone(), pool);
    let archive = directory.path().join("export.car.zst");
    let export_response = server
        .handle_request(IpcRequest::new(IpcCommand::ExportArchive {
            namespace: folder.namespace_id().to_string(),
            version: None,
            output: archive.clone(),
        }))
        .await;
    ensure!(matches!(export_response, IpcResponse::ExportComplete(_)));

    let target = directory.path().join("imported");
    let import_response = server
        .handle_request(IpcRequest::new(IpcCommand::ImportArchive {
            input: archive,
            target: target.clone(),
            filter: None,
        }))
        .await;
    ensure!(matches!(import_response, IpcResponse::ImportComplete(_)));
    ensure!(target.join("daemon.txt").is_file());
    ensure!(fs::read(target.join("daemon.txt"))? == content);
    node.stop().await?;
    Ok(())
}
