use std::path::Path;

use anyhow::Context;
use syncweb_core::{
    folder::{FolderManager, SyncMode},
    sync::{
        FetchFilter, FetchStrategy, conflict_path, diff_path, list_conflicts, materialize_folder_content,
        materialize_selected_content, resolve_conflict_artifacts, select_winner,
    },
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
    let loser_variant_hash = variants
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

    let conflict = mount_root.join(conflict_path(Path::new("binary.dat"), loser_variant_hash));
    let loser_bytes_from_store = node.blob_store().get(loser_variant_hash).await?;
    anyhow::ensure!(tokio::fs::read(&conflict).await? == loser_bytes_from_store.as_ref());
    anyhow::ensure!(!mount_root.join("binary.dat.diff").exists());

    let removed = resolve_conflict_artifacts(&node, &folder, &mount_root, None).await?;
    anyhow::ensure!(removed == 1);
    anyhow::ensure!(mount_root.join("binary.dat").exists());
    anyhow::ensure!(!conflict.exists());
    anyhow::ensure!(resolve_conflict_artifacts(&node, &folder, &mount_root, None).await? == 0);

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn selected_download_materializes_conflict_artifacts() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-selected-conflict-test")?;
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
    let loser_variant_hash = variants
        .iter()
        .map(|entry| entry.content_hash())
        .find(|hash| *hash != winner.content_hash())
        .context("loser should have a distinct hash")?;

    let mount_root = directory.path().join("materialized");
    let strategy = FetchStrategy::Filter(FetchFilter::new().with_paths(vec!["binary.dat".into()]));
    let materialized = materialize_selected_content(&node, &folder, &strategy, &mount_root).await?;
    anyhow::ensure!(materialized == 2);
    anyhow::ensure!(mount_root.join("binary.dat").exists());
    anyhow::ensure!(
        mount_root
            .join(conflict_path(Path::new("binary.dat"), loser_variant_hash))
            .exists()
    );

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn text_divergence_materializes_a_compact_diff() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-text-conflict-test")?;
    let node = test_node(&directory, "node").await?;
    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::SendReceive).await?;

    let winner_bytes = format!("{}\nold value\n{}", "unchanged\n".repeat(20), "unchanged\n".repeat(20)).into_bytes();
    let loser_bytes = format!("{}\nnew value\n{}", "unchanged\n".repeat(20), "unchanged\n".repeat(20)).into_bytes();
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
            "note.txt",
            winner_hash,
            u64::try_from(winner_bytes.len())?,
        )
        .await?;
    node.docs_engine()
        .set_blob(
            folder.doc(),
            second_author,
            "note.txt",
            loser_hash,
            u64::try_from(loser_bytes.len())?,
        )
        .await?;

    let mount_root = directory.path().join("materialized");
    let outcome = materialize_folder_content(&node, &manager, folder.namespace_id(), &mount_root).await?;
    anyhow::ensure!(outcome.summary.materialized == 1);
    anyhow::ensure!(outcome.summary.conflicts == 1);
    anyhow::ensure!(mount_root.join(diff_path(Path::new("note.txt"))).is_file());
    anyhow::ensure!(
        !mount_root
            .join(conflict_path(Path::new("note.txt"), loser_hash))
            .exists()
    );

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn clean_three_way_merge_materializes_a_new_document_version() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-three-way-conflict-test")?;
    let node = test_node(&directory, "node").await?;
    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::SendReceive).await?;

    let base = b"one\ntwo\nthree\n";
    let base_hash = folder.set_blob("note.txt", base).await?;
    let mount_root = directory.path().join("materialized");
    let initial = materialize_folder_content(&node, &manager, folder.namespace_id(), &mount_root).await?;
    anyhow::ensure!(initial.summary.materialized == 1);
    anyhow::ensure!(
        node.node_db()
            .load_conflict_ancestor(&folder.namespace_id().to_string(), b"note.txt")?
            .is_some_and(|(hash, _)| hash == base_hash)
    );

    let left = b"ONE\ntwo\nthree\n";
    let right = b"one\ntwo\nTHREE\n";
    let left_hash = node.blob_store().add_bytes(left).await?;
    let right_hash = node.blob_store().add_bytes(right).await?;
    let mut rng = rand::rng();
    let left_author = node
        .docs_engine()
        .import_author(iroh_docs::Author::new(&mut rng))
        .await?;
    let right_author = node
        .docs_engine()
        .import_author(iroh_docs::Author::new(&mut rng))
        .await?;
    node.docs_engine()
        .set_blob(
            folder.doc(),
            left_author,
            "note.txt",
            left_hash,
            u64::try_from(left.len())?,
        )
        .await?;
    node.docs_engine()
        .set_blob(
            folder.doc(),
            right_author,
            "note.txt",
            right_hash,
            u64::try_from(right.len())?,
        )
        .await?;

    let expected_hash = node.blob_store().add_bytes(b"ONE\ntwo\nTHREE\n").await?;
    let outcome = materialize_folder_content(&node, &manager, folder.namespace_id(), &mount_root).await?;
    anyhow::ensure!(outcome.summary.materialized == 1);
    anyhow::ensure!(outcome.summary.conflicts == 0);
    anyhow::ensure!(tokio::fs::read(mount_root.join("note.txt")).await? == b"ONE\ntwo\nTHREE\n");
    let entries = folder.docs_engine().list_all_entries(folder.doc()).await?;
    let variants: Vec<_> = entries
        .iter()
        .filter(|entry| entry.key() == b"note.txt")
        .cloned()
        .collect();
    anyhow::ensure!(variants.len() == 3);
    anyhow::ensure!(select_winner(&variants).is_some_and(|entry| entry.content_hash() == expected_hash));
    anyhow::ensure!(
        node.node_db()
            .load_conflict_ancestor(&folder.namespace_id().to_string(), b"note.txt")?
            .is_some_and(|(hash, _)| hash == expected_hash)
    );
    let repeat = materialize_folder_content(&node, &manager, folder.namespace_id(), &mount_root).await?;
    anyhow::ensure!(repeat.summary.conflicts == 0);

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn unsafe_logical_path_is_rejected_before_materialization() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-unsafe-conflict-path-test")?;
    let node = test_node(&directory, "node").await?;
    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::SendReceive).await?;
    let bytes = b"unsafe path".to_vec();
    let hash = node.blob_store().add_bytes(&bytes).await?;

    node.docs_engine()
        .set_blob(
            folder.doc(),
            folder.author(),
            "../escape.txt",
            hash,
            u64::try_from(bytes.len())?,
        )
        .await?;

    let mount_root = directory.path().join("materialized");
    let result = materialize_folder_content(&node, &manager, folder.namespace_id(), &mount_root).await;
    anyhow::ensure!(result.is_err(), "path traversal key must not be materialized");
    anyhow::ensure!(!directory.path().join("escape.txt").exists());

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn receiveonly_conflict_is_reported_without_materializing_loser() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-receiveonly-conflict-test")?;
    let node = test_node(&directory, "node").await?;
    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::ReceiveOnly).await?;

    let winner_bytes = b"winner bytes".to_vec();
    let loser_bytes = b"loser bytes".to_vec();
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
            "note.txt",
            winner_hash,
            u64::try_from(winner_bytes.len())?,
        )
        .await?;
    node.docs_engine()
        .set_blob(
            folder.doc(),
            second_author,
            "note.txt",
            loser_hash,
            u64::try_from(loser_bytes.len())?,
        )
        .await?;

    let entries = folder.docs_engine().list_all_entries(folder.doc()).await?;
    let reports = list_conflicts(&node, entries.clone()).await?;
    anyhow::ensure!(reports.len() == 1);
    let report = reports.first().context("expected one conflict report")?;
    anyhow::ensure!(report.path == "note.txt");
    anyhow::ensure!(report.variants.iter().filter(|variant| variant.winner).count() == 1);

    let variants: Vec<_> = entries.iter().filter(|entry| entry.key() == b"note.txt").collect();
    let variant_entries: Vec<_> = variants.iter().map(|entry| (*entry).clone()).collect();
    let winner = select_winner(&variant_entries).context("winner should be selected")?;
    let loser_variant_hash = variants
        .iter()
        .map(|entry| entry.content_hash())
        .find(|hash| *hash != winner.content_hash())
        .context("loser should have a distinct hash")?;
    let winner_bytes_from_store = node.blob_store().get(winner.content_hash()).await?;

    let mount_root = directory.path().join("materialized");
    let outcome = materialize_folder_content(&node, &manager, folder.namespace_id(), &mount_root).await?;
    anyhow::ensure!(outcome.summary.materialized == 1);
    anyhow::ensure!(outcome.summary.conflicts == 0);
    anyhow::ensure!(tokio::fs::read(mount_root.join("note.txt")).await? == winner_bytes_from_store.as_ref());
    anyhow::ensure!(!mount_root.join(diff_path(Path::new("note.txt"))).exists());
    anyhow::ensure!(
        !mount_root
            .join(conflict_path(Path::new("note.txt"), loser_variant_hash))
            .exists()
    );

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn metadata_only_conflict_stays_at_document_level_until_content_is_local() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-metadata-only-conflict-test")?;
    let node = test_node(&directory, "node").await?;
    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::SendReceive).await?;
    folder.set_metadata_only().await?;

    let winner_hash = iroh_blobs::Hash::from_bytes([1; 32]);
    let loser_hash = iroh_blobs::Hash::from_bytes([2; 32]);
    let first_author = folder.author();
    let mut rng = rand::rng();
    let second_author = node
        .docs_engine()
        .import_author(iroh_docs::Author::new(&mut rng))
        .await?;
    node.docs_engine()
        .set_blob(folder.doc(), first_author, "note.txt", winner_hash, 12)
        .await?;
    node.docs_engine()
        .set_blob(folder.doc(), second_author, "note.txt", loser_hash, 11)
        .await?;

    let entries = folder.docs_engine().list_all_entries(folder.doc()).await?;
    let reports = list_conflicts(&node, entries).await?;
    anyhow::ensure!(reports.len() == 1);
    let report = reports.first().context("expected one conflict report")?;
    anyhow::ensure!(report.variants.iter().all(|variant| !variant.local));

    let mount_root = directory.path().join("materialized");
    let outcome = materialize_folder_content(&node, &manager, folder.namespace_id(), &mount_root).await?;
    anyhow::ensure!(outcome.summary.materialized == 0);
    anyhow::ensure!(outcome.summary.conflicts == 0);
    anyhow::ensure!(outcome.summary.pending == 1);
    anyhow::ensure!(!mount_root.exists());

    node.stop().await?;
    Ok(())
}
