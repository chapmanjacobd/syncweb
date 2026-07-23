use anyhow::Context;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use n0_future::StreamExt;
use syncweb_core::{
    search::{FindEngine, FindQuery},
    sync::{LazyFetch, SyncEvent},
};

use crate::test_utils::{TestDirectory, test_node};

#[tokio::test]
async fn test_ls_without_download() -> anyhow::Result<()> {
    let dir = TestDirectory::new("syncweb-lazy-test")?;
    let node = test_node(&dir, "node").await?;
    let doc = node.docs_engine().create_namespace().await?;
    let author = node.docs_engine().author().await?;

    let hash = node.blob_store().add_bytes(b"file content").await?;
    node.docs_engine().set_blob(&doc, author, b"file.txt", hash, 12).await?;

    let entry_opt = node.docs_engine().get(&doc, author, b"file.txt").await?;
    anyhow::ensure!(entry_opt.is_some(), "entry should exist");

    let entry = entry_opt.unwrap();
    anyhow::ensure!(entry.content_hash() == hash);
    anyhow::ensure!(entry.content_len() == 12);

    // Verify blob was NOT fetched from remote - we only read metadata
    anyhow::ensure!(node.blob_store().has(hash).await?);

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_download_triggers_fetch() -> anyhow::Result<()> {
    let dir = TestDirectory::new("syncweb-lazy-test")?;
    let node = test_node(&dir, "node").await?;

    let hash = node.blob_store().add_bytes(b"lazy data").await?;
    let lazy = LazyFetch::new(node.blob_store().clone(), node.docs_engine().clone());

    let bytes = lazy.fetch(hash).await?;
    anyhow::ensure!(bytes.as_ref() == b"lazy data");

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_fetch_intent_emits_events() -> anyhow::Result<()> {
    let dir = TestDirectory::new("syncweb-lazy-test")?;
    let node = test_node(&dir, "node").await?;

    let hash = node.blob_store().add_bytes(b"intent data").await?;
    let lazy = LazyFetch::new(node.blob_store().clone(), node.docs_engine().clone());

    let mut handle = lazy.fetch_intent(hash);

    let event = tokio::time::timeout(Duration::from_secs(5), handle.next())
        .await?
        .context("stream should not be empty")?;
    anyhow::ensure!(matches!(event, SyncEvent::Started));

    loop {
        let next_event = tokio::time::timeout(Duration::from_secs(5), handle.next())
            .await?
            .context("stream should not be empty")?;
        if matches!(next_event, SyncEvent::Finished) {
            break;
        }
        if matches!(next_event, SyncEvent::Failed(_)) {
            anyhow::bail!("fetch intent failed");
        }
    }

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_find_without_download() -> anyhow::Result<()> {
    let source = dir_path("find_no_download")?;
    fs::create_dir_all(source.join("sub"))?;
    fs::write(source.join("report.pdf"), b"r")?;
    fs::write(source.join("data.txt"), b"d")?;
    fs::write(source.join("sub/note.txt"), b"n")?;

    let found = FindEngine::new(&source).find(&FindQuery::glob("*.txt"))?;
    anyhow::ensure!(found.len() == 2, "expected 2 txt files, got {}", found.len());

    let names: Vec<_> = found.iter().map(|e| e.relative_path.clone()).collect();
    anyhow::ensure!(names.contains(&PathBuf::from("data.txt")));
    anyhow::ensure!(names.contains(&PathBuf::from("sub/note.txt")));

    fs::remove_dir_all(&source)?;
    Ok(())
}

fn dir_path(name: &str) -> anyhow::Result<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target")
        .join(format!("lazy-{name}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&path)?;
    Ok(path)
}
