use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use syncweb_core::search::{FindEngine, FindQuery};

fn test_root(name: &str) -> anyhow::Result<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target")
        .join(format!("phase3-find-{name}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root)?;
    Ok(root)
}

#[test]
fn test_exact_match() -> anyhow::Result<()> {
    let root = test_root("exact")?;
    fs::write(root.join("report.txt"), b"r")?;
    fs::write(root.join("summary.txt"), b"s")?;

    let found = FindEngine::new(&root).find(&FindQuery::exact("report"))?;
    anyhow::ensure!(found.len() == 1, "expected 1, got {}", found.len());
    anyhow::ensure!(found.first().unwrap().relative_path == Path::new("report.txt"));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_glob_match() -> anyhow::Result<()> {
    let root = test_root("glob")?;
    fs::write(root.join("main.rs"), b"m")?;
    fs::write(root.join("lib.rs"), b"l")?;
    fs::write(root.join("readme.md"), b"r")?;

    let found = FindEngine::new(&root).find(&FindQuery::glob("*.rs"))?;
    anyhow::ensure!(found.len() == 2, "expected 2, got {}", found.len());

    let names: Vec<_> = found.iter().map(|e| e.relative_path.clone()).collect();
    anyhow::ensure!(names.contains(&PathBuf::from("lib.rs")));
    anyhow::ensure!(names.contains(&PathBuf::from("main.rs")));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_regex_match() -> anyhow::Result<()> {
    let root = test_root("regex")?;
    fs::write(root.join("report-01.pdf"), b"a")?;
    fs::write(root.join("report-42.pdf"), b"b")?;
    fs::write(root.join("report-abc.pdf"), b"c")?;
    fs::write(root.join("other.txt"), b"d")?;

    let found = FindEngine::new(&root).find(&FindQuery::regex(r"report-\d+\.pdf"))?;
    anyhow::ensure!(found.len() == 2, "expected 2, got {}", found.len());

    let names: Vec<_> = found.iter().map(|e| e.relative_path.clone()).collect();
    anyhow::ensure!(names.contains(&PathBuf::from("report-01.pdf")));
    anyhow::ensure!(names.contains(&PathBuf::from("report-42.pdf")));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_depth_filter() -> anyhow::Result<()> {
    let root = test_root("depth")?;
    fs::write(root.join("top.txt"), b"t")?;
    fs::create_dir_all(root.join("a"))?;
    fs::write(root.join("a/mid.txt"), b"m")?;
    fs::create_dir_all(root.join("a/b"))?;
    fs::write(root.join("a/b/deep.txt"), b"d")?;

    let found = FindEngine::new(&root).find(&FindQuery::glob("*").depth(2))?;
    anyhow::ensure!(found.len() == 2, "depth 2 should return 2, got {}", found.len());

    let found_root = FindEngine::new(&root).find(&FindQuery::glob("*").depth(1))?;
    anyhow::ensure!(
        found_root.len() == 1,
        "depth 1 should return 1, got {}",
        found_root.len()
    );

    let found_all = FindEngine::new(&root).find(&FindQuery::glob("*").depth(3))?;
    anyhow::ensure!(found_all.len() == 3, "depth 3 should return 3, got {}", found_all.len());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_size_filter() -> anyhow::Result<()> {
    let root = test_root("size")?;
    fs::write(root.join("small.txt"), b"ab")?;
    fs::write(root.join("medium.txt"), b"abcdefghij")?;
    fs::write(root.join("large.txt"), [0_u8; 100])?;

    let found = FindEngine::new(&root).find(&FindQuery::glob("*").size(Some(5), Some(50)))?;
    anyhow::ensure!(found.len() == 1, "expected 1, got {}", found.len());
    anyhow::ensure!(found.first().unwrap().relative_path == Path::new("medium.txt"));

    let found_large = FindEngine::new(&root).find(&FindQuery::glob("*").size(Some(50), None))?;
    anyhow::ensure!(found_large.len() == 1, "expected 1 large, got {}", found_large.len());
    anyhow::ensure!(found_large.first().unwrap().relative_path == Path::new("large.txt"));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_time_filter() -> anyhow::Result<()> {
    let root = test_root("time")?;
    fs::write(root.join("old.txt"), b"old")?;
    fs::write(root.join("new.txt"), b"new")?;

    let entries = syncweb_core::fs::Scanner::new(&root, Vec::<String>::new()).scan()?;
    let old_time = entries
        .iter()
        .find(|e| e.relative_path == Path::new("old.txt"))
        .map_or(SystemTime::UNIX_EPOCH, |e| e.modified);

    let found = FindEngine::new(&root)
        .find(&FindQuery::glob("*").modified_after(old_time + std::time::Duration::from_secs(1)))?;
    anyhow::ensure!(
        found.iter().all(|e| e.relative_path == Path::new("new.txt")),
        "only new.txt should match after old_time"
    );

    let found_all = FindEngine::new(&root).find(&FindQuery::glob("*"))?;
    anyhow::ensure!(found_all.len() == 2, "without time filter should return 2");

    fs::remove_dir_all(root)?;
    Ok(())
}
