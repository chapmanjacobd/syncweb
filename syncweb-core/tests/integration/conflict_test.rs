use std::path::Path;

use anyhow::Context;
use syncweb_core::{
    folder::{FolderManager, SyncMode},
    sync::{conflict_path, materialize_folder_content, select_winner},
};

use crate::{integration::test_node, test_utils::TestDirectory};

#[tokio::test]
async fn binary_divergence_materializes_a_full_conflict_copy() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-conflict-test")?;
    let node = test_node(&directory, "node").await?;
    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::SendReceive).await?;

    let winner_bytes = b"winner\0bytes".to_vec();
    let loser_bytes = b"loser\0bytes".to_vec();
    let winner_hash = node.blob_store().add_bytes(&winner_bytes).await?;
    let loser_hash = node.blob_store().add_bytes(&loser_bytes).await?;
    let first_author = folder.author();
    let mut rng = rand::rng();
    let second_author = node
        .docs_engine()
        .import_author(iroh_docs::Author::new(&mut rng))
        .await?;

    node.docs_engine()
        .set_blob(
            folder.doc(),
            first_author,
            "binary.dat",
            winner_hash,
            u64::try_from(winner_bytes.len())?,
        )
        .await?;
    node.docs_engine()
        .set_blob(
            folder.doc(),
            second_author,
            "binary.dat",
            loser_hash,
            u64::try_from(loser_bytes.len())?,
        )
        .await?;

    let entries = folder.docs_engine().list_all_entries(folder.doc()).await?;
    let variants: Vec<_> = entries.iter().filter(|entry| entry.key() == b"binary.dat").collect();
    anyhow::ensure!(variants.len() == 2, "expected two divergent binary variants");
    let variant_entries: Vec<_> = variants.iter().map(|entry| (*entry).clone()).collect();
    let winner = select_winner(&variant_entries).context("winner should be selected")?;
    let loser_hash = variants
        .iter()
        .map(|entry| entry.content_hash())
        .find(|hash| *hash != winner.content_hash())
        .context("loser should have a distinct hash")?;

    let mount_root = directory.path().join("materialized");
    let outcome = materialize_folder_content(&node, &manager, folder.namespace_id(), &mount_root).await?;
    anyhow::ensure!(outcome.summary.materialized == 1);
    anyhow::ensure!(outcome.summary.conflicts == 1);

    let canonical = tokio::fs::read(mount_root.join("binary.dat")).await?;
    let winner_bytes_from_store = node.blob_store().get(winner.content_hash()).await?;
    anyhow::ensure!(canonical == winner_bytes_from_store.as_ref());

    let conflict = mount_root.join(conflict_path(Path::new("binary.dat"), loser_hash));
    let loser_bytes_from_store = node.blob_store().get(loser_hash).await?;
    anyhow::ensure!(tokio::fs::read(conflict).await? == loser_bytes_from_store.as_ref());
    anyhow::ensure!(!mount_root.join("binary.dat.diff").exists());

    node.stop().await?;
    Ok(())
}
