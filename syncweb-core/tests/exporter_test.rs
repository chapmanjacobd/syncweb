use std::{
    fs,
    path::{Path, PathBuf},
};

use syncweb_core::{
    fs::{ExportEntry, Exporter, ParallelExporter},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Result<Self, std::io::Error> {
        let path = std::env::temp_dir().join(format!("syncweb-exporter-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("failed to remove test directory {}: {error}", self.0.display());
        }
    }
}

async fn test_node(directory: &TestDirectory, name: &str) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new(identity, root.join("data"), RelayMode::Default).await?)
}

#[tokio::test]
async fn test_export_single_blob() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let node = test_node(&dir, "node").await?;

    let hash = node.blob_store().add_bytes(b"export me").await?;

    let dest = dir.path().join("export_dest");
    let exporter = Exporter::new(node.blob_store().clone(), &dest);

    let entry = ExportEntry::new("output.txt", hash, 9);

    let path = exporter.export_entry(&entry).await?;
    anyhow::ensure!(path.exists(), "exported file should exist");
    anyhow::ensure!(fs::read(&path)? == b"export me");

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_export_directory() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let node = test_node(&dir, "node").await?;

    let h1 = node.blob_store().add_bytes(b"alpha").await?;
    let h2 = node.blob_store().add_bytes(b"beta").await?;

    let dest = dir.path().join("export_dest");
    let exporter = Exporter::new(node.blob_store().clone(), &dest);

    let entries = vec![ExportEntry::new("a.txt", h1, 5), ExportEntry::new("sub/b.txt", h2, 4)];

    let paths = exporter.export(&entries).await?;
    anyhow::ensure!(paths.len() == 2, "expected 2 exported files");

    let a_path = dest.join("a.txt");
    let b_path = dest.join("sub/b.txt");
    anyhow::ensure!(a_path.exists(), "a.txt should exist");
    anyhow::ensure!(b_path.exists(), "sub/b.txt should exist");
    anyhow::ensure!(fs::read(&a_path)? == b"alpha");
    anyhow::ensure!(fs::read(&b_path)? == b"beta");

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_parallel_export() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let node = test_node(&dir, "node").await?;

    let h1 = node.blob_store().add_bytes(b"one").await?;
    let h2 = node.blob_store().add_bytes(b"two").await?;
    let h3 = node.blob_store().add_bytes(b"three").await?;

    let dest = dir.path().join("export_dest");
    let exporter = ParallelExporter::new(node.blob_store().clone(), &dest).with_threads(2);

    let entries = vec![
        ExportEntry::new("1.txt", h1, 3),
        ExportEntry::new("2.txt", h2, 3),
        ExportEntry::new("3.txt", h3, 5),
    ];

    let paths = exporter.export(&entries).await?;
    anyhow::ensure!(paths.len() == 3);

    anyhow::ensure!(fs::read(dest.join("1.txt"))? == b"one");
    anyhow::ensure!(fs::read(dest.join("2.txt"))? == b"two");
    anyhow::ensure!(fs::read(dest.join("3.txt"))? == b"three");

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_export_verify_hash() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let node = test_node(&dir, "node").await?;

    let hash = node.blob_store().add_bytes(b"verify me").await?;

    let dest = dir.path().join("export_dest");
    let exporter = Exporter::new(node.blob_store().clone(), &dest);

    let entry = ExportEntry::new("verified.txt", hash, 9);

    let path = exporter.export_verified(&entry).await?;
    anyhow::ensure!(path.exists());

    let actual = blake3::hash(&fs::read(&path)?);
    anyhow::ensure!(
        actual.as_bytes() == hash.as_bytes(),
        "hash should match after verified export"
    );

    node.stop().await?;
    Ok(())
}
