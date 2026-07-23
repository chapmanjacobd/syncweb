mod test_utils;

use std::{fs, path::PathBuf};

use syncweb_core::fs::{Importer, ParallelImporter};

use crate::test_utils::{TestDirectory, test_node};

#[tokio::test]
async fn test_import_single_file() -> anyhow::Result<()> {
    let dir = TestDirectory::new("syncweb-importer-test")?;
    let node = test_node(&dir, "node").await?;
    let doc = node.docs_engine().create_namespace().await?;
    let author = node.docs_engine().author().await?;

    let source = dir.path().join("source");
    fs::create_dir_all(&source)?;
    fs::write(source.join("hello.txt"), b"hello world")?;

    let importer = Importer::new(
        node.blob_store().clone(),
        node.docs_engine().clone(),
        doc.clone(),
        author,
    );
    let entries = importer.import_path(&source).await?;

    anyhow::ensure!(entries.len() == 1, "expected 1 import entry, got {}", entries.len());
    let first = entries.first().unwrap();
    anyhow::ensure!(first.relative_path == PathBuf::from("hello.txt"));
    anyhow::ensure!(first.size == 11);
    anyhow::ensure!(node.blob_store().has(first.hash).await?);

    let entry = node.docs_engine().get(&doc, author, b"hello.txt").await?;
    anyhow::ensure!(entry.is_some(), "doc entry should exist after import");

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_import_directory() -> anyhow::Result<()> {
    let dir = TestDirectory::new("syncweb-importer-test")?;
    let node = test_node(&dir, "node").await?;
    let doc = node.docs_engine().create_namespace().await?;
    let author = node.docs_engine().author().await?;

    let source = dir.path().join("source");
    fs::create_dir_all(source.join("sub"))?;
    fs::write(source.join("a.txt"), b"alpha")?;
    fs::write(source.join("sub/b.txt"), b"beta")?;

    let importer = Importer::new(
        node.blob_store().clone(),
        node.docs_engine().clone(),
        doc.clone(),
        author,
    );
    let entries = importer.import_path(&source).await?;

    anyhow::ensure!(entries.len() == 2, "expected 2 import entries, got {}", entries.len());

    let paths: Vec<_> = entries.iter().map(|e| e.relative_path.clone()).collect();
    anyhow::ensure!(paths.contains(&PathBuf::from("a.txt")));
    anyhow::ensure!(paths.contains(&PathBuf::from("sub/b.txt")));

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_parallel_import() -> anyhow::Result<()> {
    let dir = TestDirectory::new("syncweb-importer-test")?;
    let node = test_node(&dir, "node").await?;
    let doc = node.docs_engine().create_namespace().await?;
    let author = node.docs_engine().author().await?;

    let source = dir.path().join("source");
    fs::create_dir_all(&source)?;
    fs::write(source.join("one.txt"), b"one")?;
    fs::write(source.join("two.txt"), b"two")?;

    let importer = ParallelImporter::new(
        node.blob_store().clone(),
        node.docs_engine().clone(),
        doc.clone(),
        author,
    )
    .with_threads(2);
    let entries = importer.import_path(&source).await?;

    anyhow::ensure!(entries.len() == 2, "expected 2, got {}", entries.len());

    let mut hashes: Vec<_> = entries.iter().map(|e| e.hash).collect();
    hashes.sort();
    let mut expected = vec![blake3::hash(b"one").into(), blake3::hash(b"two").into()];
    expected.sort();
    anyhow::ensure!(hashes == expected, "hashes should match content");

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_import_idempotent() -> anyhow::Result<()> {
    let dir = TestDirectory::new("syncweb-importer-test")?;
    let node = test_node(&dir, "node").await?;
    let doc = node.docs_engine().create_namespace().await?;
    let author = node.docs_engine().author().await?;

    let source = dir.path().join("source");
    fs::create_dir_all(&source)?;
    fs::write(source.join("file.txt"), b"content")?;

    let importer = Importer::new(
        node.blob_store().clone(),
        node.docs_engine().clone(),
        doc.clone(),
        author,
    );

    let first = importer.import_path(&source).await?;
    let first_hash = first.first().unwrap().hash;

    let second = importer.import_path(&source).await?;
    let second_hash = second.first().unwrap().hash;

    anyhow::ensure!(
        first_hash == second_hash,
        "re-importing same file should produce same hash"
    );
    anyhow::ensure!(node.blob_store().has(first_hash).await?, "blob should exist");

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_import_rejects_stale_scanned_entry() -> anyhow::Result<()> {
    let dir = TestDirectory::new("syncweb-importer-test")?;
    let node = test_node(&dir, "node").await?;
    let doc = node.docs_engine().create_namespace().await?;
    let author = node.docs_engine().author().await?;

    let source = dir.path().join("source");
    fs::create_dir_all(&source)?;
    let path = source.join("file.txt");
    fs::write(&path, b"before")?;

    let scanner = syncweb_core::fs::Scanner::new(&source, Vec::<String>::new());
    let mut entries = scanner.scan()?;
    fs::write(&path, b"after")?;

    let importer = Importer::new(
        node.blob_store().clone(),
        node.docs_engine().clone(),
        doc.clone(),
        author,
    );
    let error = importer.import_entries(std::mem::take(&mut entries)).await.unwrap_err();
    anyhow::ensure!(error.to_string().contains("file changed during import"));
    anyhow::ensure!(node.docs_engine().get(&doc, author, b"file.txt").await?.is_none());

    node.stop().await?;
    Ok(())
}
