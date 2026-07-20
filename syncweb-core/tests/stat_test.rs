use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use syncweb_core::{
    fs::{FileEntry, FileType, Scanner},
    stat::{StatFormat, StatOutput},
};

fn test_root(name: &str) -> anyhow::Result<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target")
        .join(format!("phase3-stat-{name}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root)?;
    Ok(root)
}

#[test]
fn test_stat_file() -> anyhow::Result<()> {
    let root = test_root("file")?;
    fs::write(root.join("data.txt"), b"hello world")?;

    let entries = Scanner::new(&root, Vec::<String>::new()).scan()?;
    anyhow::ensure!(!entries.is_empty(), "should have at least one entry");

    let entry = entries.iter().find(|e| e.relative_path == Path::new("data.txt"));
    anyhow::ensure!(entry.is_some(), "data.txt should be in scan results");

    let stat = StatOutput::from_entry(entry.unwrap());
    anyhow::ensure!(stat.size == 11);
    anyhow::ensure!(stat.available);
    anyhow::ensure!(stat.hash.is_some());
    anyhow::ensure!(stat.hash.unwrap() == blake3::hash(b"hello world").into());
    anyhow::ensure!(stat.version_vector == BTreeMap::new());
    anyhow::ensure!(stat.blocks() == 1);

    let human = stat.display(StatFormat::Human);
    anyhow::ensure!(human.contains("Size: 11"));
    anyhow::ensure!(human.contains("Blocks: 1"));
    anyhow::ensure!(human.contains("Available: true"));

    let terse = stat.display(StatFormat::Terse);
    anyhow::ensure!(terse.contains("11"));
    anyhow::ensure!(terse.contains('|'));

    let custom = stat.display(StatFormat::Custom("%n|%s|%b".to_owned()));
    anyhow::ensure!(custom.contains("data.txt"));
    anyhow::ensure!(custom.contains("11"));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_stat_folder() -> anyhow::Result<()> {
    let root = test_root("folder")?;
    fs::create_dir_all(root.join("sub"))?;
    fs::write(root.join("a.txt"), b"aaa")?;
    fs::write(root.join("sub/b.txt"), b"bb")?;

    let entries = Scanner::new(&root, Vec::<String>::new()).scan()?;
    let stats: Vec<StatOutput> = entries.iter().map(StatOutput::from_entry).collect();

    anyhow::ensure!(stats.len() == 2, "expected 2 file stats, got {}", stats.len());

    let total_size: u64 = stats.iter().map(|s| s.size).sum();
    anyhow::ensure!(total_size == 5, "total size should be 5, got {total_size}");

    let total_blocks: u64 = stats.iter().map(StatOutput::blocks).sum();
    anyhow::ensure!(
        total_blocks == 2,
        "both files fit in 1 block each, total should be 2, got {total_blocks}"
    );

    for stat in &stats {
        anyhow::ensure!(stat.available);
        anyhow::ensure!(stat.hash.is_some());
        anyhow::ensure!(stat.peers == 0);
    }

    let dir_entry = FileEntry::builder()
        .path(root.clone())
        .relative_path(PathBuf::from("."))
        .size(0)
        .modified(SystemTime::now())
        .hash(blake3::hash(&[]))
        .file_type(FileType::Directory)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;
    let dir_stat = StatOutput::from_entry(&dir_entry);
    anyhow::ensure!(dir_stat.size == 0);
    anyhow::ensure!(dir_stat.blocks() == 0);

    fs::remove_dir_all(root)?;
    Ok(())
}
