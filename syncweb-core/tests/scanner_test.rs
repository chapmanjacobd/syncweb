use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use syncweb_core::fs::{ParallelScanner, Scanner};

fn test_root(name: &str) -> std::io::Result<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target")
        .join(format!("scanner-{name}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root)?;
    Ok(root)
}

#[test]
fn test_scan_empty_dir() -> anyhow::Result<()> {
    let root = test_root("empty")?;
    let entries = Scanner::new(&root, Vec::<String>::new()).scan()?;
    anyhow::ensure!(entries.is_empty(), "empty dir should produce no entries");
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_scan_single_file() -> anyhow::Result<()> {
    let root = test_root("single")?;
    fs::write(root.join("hello.txt"), b"hello")?;

    let entries = Scanner::new(&root, Vec::<String>::new()).scan()?;
    anyhow::ensure!(entries.len() == 1, "expected 1 entry, got {}", entries.len());
    let first = entries.first().unwrap();
    anyhow::ensure!(first.relative_path == Path::new("hello.txt"));
    anyhow::ensure!(first.size == 5);
    anyhow::ensure!(first.hash == blake3::hash(b"hello"));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_scan_nested_dirs() -> anyhow::Result<()> {
    let root = test_root("nested")?;
    fs::create_dir_all(root.join("a/b/c"))?;
    fs::write(root.join("top.txt"), b"t")?;
    fs::write(root.join("a/mid.txt"), b"m")?;
    fs::write(root.join("a/b/deep.txt"), b"d")?;
    fs::write(root.join("a/b/c/deepest.txt"), b"x")?;

    let entries = Scanner::new(&root, Vec::<String>::new()).scan()?;
    anyhow::ensure!(entries.len() == 4, "expected 4 entries, got {}", entries.len());

    let paths: Vec<_> = entries.iter().map(|e| e.relative_path.clone()).collect();
    anyhow::ensure!(
        paths
            == [
                PathBuf::from("a/b/c/deepest.txt"),
                PathBuf::from("a/b/deep.txt"),
                PathBuf::from("a/mid.txt"),
                PathBuf::from("top.txt"),
            ],
        "paths should be sorted: {paths:?}"
    );

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_parallel_vs_sequential() -> anyhow::Result<()> {
    let root = test_root("parallel")?;
    fs::create_dir_all(root.join("nested"))?;
    fs::write(root.join("one.txt"), b"one")?;
    fs::write(root.join("nested/two.txt"), b"two")?;
    fs::write(root.join("skip.tmp"), b"skip")?;

    let sequential = Scanner::new(&root, vec!["*.tmp".to_owned()]).scan()?;
    let parallel = ParallelScanner::new(&root, vec!["*.tmp".to_owned()], 2).scan()?;

    anyhow::ensure!(sequential.len() == 2, "expected 2, got {}", sequential.len());

    let seq_paths: Vec<_> = sequential.iter().map(|entry| &entry.relative_path).collect();
    let par_paths: Vec<_> = parallel.iter().map(|entry| &entry.relative_path).collect();
    anyhow::ensure!(
        seq_paths == par_paths,
        "paths do not match: {seq_paths:?} != {par_paths:?}"
    );

    let one = sequential
        .iter()
        .find(|entry| entry.relative_path == Path::new("one.txt"))
        .map(|entry| entry.hash);
    let expected_hash = Some(blake3::hash(b"one"));
    anyhow::ensure!(
        one == expected_hash,
        "hash mismatch: expected {expected_hash:?}, got {one:?}"
    );

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_ignore_patterns() -> anyhow::Result<()> {
    let root = test_root("ignore")?;
    fs::write(root.join("keep.txt"), b"k")?;
    fs::write(root.join("skip.tmp"), b"s")?;
    fs::write(root.join("also.log"), b"l")?;
    fs::create_dir_all(root.join("node_modules"))?;
    fs::write(root.join("node_modules/pkg.js"), b"p")?;

    let entries = Scanner::new(
        &root,
        vec!["*.tmp".to_owned(), "*.log".to_owned(), "node_modules".to_owned()],
    )
    .scan()?;
    anyhow::ensure!(
        entries.len() == 1,
        "expected 1 entry after ignore, got {}",
        entries.len()
    );
    anyhow::ensure!(entries.first().unwrap().relative_path == Path::new("keep.txt"));

    let entries_no_nm = Scanner::new(&root, vec!["node_modules".to_owned()]).scan()?;
    anyhow::ensure!(
        entries_no_nm.len() == 3,
        "expected 3 entries ignoring node_modules, got {}",
        entries_no_nm.len()
    );

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_large_directory_perf() -> anyhow::Result<()> {
    let root = test_root("perf")?;
    let nested = root.join("deep");
    fs::create_dir_all(&nested)?;

    for i in 0..10_000 {
        fs::write(nested.join(format!("file_{i:05}.txt")), i.to_string().as_bytes())?;
    }

    let start = Instant::now();
    let entries = Scanner::new(&root, Vec::<String>::new()).scan()?;
    let elapsed = start.elapsed();

    anyhow::ensure!(entries.len() == 10_000, "expected 10000, got {}", entries.len());
    anyhow::ensure!(
        elapsed.as_millis() < 5_000,
        "scanning 10k files took {}ms, expected < 5000ms",
        elapsed.as_millis()
    );

    fs::remove_dir_all(root)?;
    Ok(())
}
