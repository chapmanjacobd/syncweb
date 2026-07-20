use std::{
    fs,
    path::{Path, PathBuf},
};

use syncweb_core::{
    search::{FindEngine, FindQuery},
    sort::{SortCriterion, SortEntry, Sorter},
};

fn test_root(name: &str) -> std::io::Result<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target")
        .join(format!("phase3-{name}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root)?;
    Ok(root)
}

#[test]
fn find_filters_and_sorter_rank_entries() -> Result<(), Box<dyn std::error::Error>> {
    let root = test_root("search")?;
    fs::create_dir_all(root.join("nested"))?;
    fs::write(root.join("report-1.txt"), b"report")?;
    fs::write(root.join("nested/other.rs"), b"source")?;

    let found = FindEngine::new(&root).find(&FindQuery::regex(r"report-\d+\.txt").extension("txt"))?;
    if found.len() != 1 {
        return Err(format!("expected 1, got {}", found.len()).into());
    }

    let mut entries = vec![SortEntry::new("common"), SortEntry::new("rare")];
    if let Some(entry) = entries.get_mut(0) {
        entry.peers = 1;
    }
    if let Some(entry) = entries.get_mut(1) {
        entry.peers = 3;
    }
    Sorter::new(SortCriterion::Peers).sort(&mut entries);
    if let Some(first) = entries.first() {
        let expected = PathBuf::from("rare");
        if first.path != expected {
            return Err(format!("expected {expected:?}, got {:?}", first.path).into());
        }
    }
    fs::remove_dir_all(root)?;
    Ok(())
}
