use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use iroh::address_lookup::memory::MemoryLookup;
use iroh_blobs::Hash;
use iroh_docs::{Entry, NamespaceId};

use syncweb_core::folder::{FolderManager, SyncMode, SyncwebFolder};
use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::{DiscoveryConfig, IrohNode, RelayMode};

use super::{TestDirectory, empty_member_keys};

pub struct World {
    _keep_alive: Box<dyn std::any::Any>,
    pub memory_lookup: MemoryLookup,
    directory: TestDirectory,
    devices: Vec<Device>,
}

pub struct Device {
    pub node: IrohNode,
    pub name: String,
    dir: PathBuf,
    _managers: HashMap<NamespaceId, FolderManager>,
}

pub struct FolderHandle {
    pub namespace: NamespaceId,
    pub folder: SyncwebFolder,
    pub manager: FolderManager,
}

impl World {
    pub async fn new(device_names: &[&str]) -> anyhow::Result<Self> {
        let directory = TestDirectory::new("syncweb-world")?;
        let (relay_map, relay_url, server) = iroh::test_utils::run_relay_server().await?;
        let memory_lookup = MemoryLookup::new();

        let mut devices = Vec::new();

        for name in device_names {
            let root = directory.path().join(name);
            let identity = IdentityManager::new(root.join("identity.key")).context("create identity")?;
            let node = IrohNode::new_with_address_lookup(
                identity,
                root.join("data"),
                RelayMode::Custom {
                    map: relay_map.clone(),
                    insecure: true,
                },
                memory_lookup.clone(),
                DiscoveryConfig::disabled(),
                empty_member_keys(),
            )
            .await
            .context("create node")?;

            memory_lookup
                .add_endpoint_info(iroh::EndpointAddr::new(node.endpoint().id()).with_relay_url(relay_url.clone()));

            devices.push(Device {
                node,
                name: name.to_string(),
                dir: root,
                _managers: HashMap::new(),
            });
        }

        Ok(Self {
            _keep_alive: Box::new((relay_map, relay_url, server)),
            memory_lookup,
            directory,
            devices,
        })
    }

    pub fn device(&self, name: &str) -> anyhow::Result<&Device> {
        self.devices.iter().find(|d| d.name == name).with_context(|| {
            let available: Vec<_> = self.devices.iter().map(|d| &d.name).collect();
            format!("device '{name}' not found; available: {available:?}")
        })
    }

    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    pub fn directory(&self) -> &Path {
        self.directory.path()
    }
}

impl Device {
    fn manager(&self) -> FolderManager {
        FolderManager::new(&self.node)
    }

    pub async fn create_folder(&self, mode: SyncMode) -> anyhow::Result<FolderHandle> {
        let manager = self.manager();
        let folder = manager.create(mode).await.context("create folder")?;
        Ok(FolderHandle {
            namespace: folder.namespace_id(),
            folder,
            manager,
        })
    }

    pub async fn join_folder(&self, ticket: &str, mode: SyncMode) -> anyhow::Result<FolderHandle> {
        let manager = self.manager();
        let folder = manager.join(ticket, mode).await.context("join folder")?;
        Ok(FolderHandle {
            namespace: folder.namespace_id(),
            folder,
            manager,
        })
    }

    pub async fn write(&self, handle: &FolderHandle, path: &str, data: &[u8]) -> anyhow::Result<Hash> {
        handle.folder.set_blob(path, data).await.context("set blob")
    }

    pub async fn get_blob(&self, hash: Hash) -> anyhow::Result<bytes::Bytes> {
        self.node.blob_store().get(hash).await.context("get blob")
    }

    pub async fn wait_entry(&self, namespace: NamespaceId, path: &str) -> anyhow::Result<Entry> {
        let manager = self.manager();
        let folder = manager.get(namespace).await.context("get folder")?;
        let doc = folder.doc().clone();

        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if let Some(entry) = self.node.docs_engine().get_any(&doc, path).await.context("get_any")? {
                    return anyhow::Ok(entry);
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        })
        .await
        .context("timed out waiting for entry")?
    }

    pub async fn list_entries(&self, namespace: NamespaceId) -> anyhow::Result<Vec<Entry>> {
        let manager = self.manager();
        let folder = manager.get(namespace).await.context("get folder")?;
        self.node
            .docs_engine()
            .list_latest(folder.doc())
            .await
            .context("list entries")
    }

    pub const fn endpoint(&self) -> &iroh::Endpoint {
        self.node.endpoint()
    }

    pub const fn node(&self) -> &IrohNode {
        &self.node
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}
