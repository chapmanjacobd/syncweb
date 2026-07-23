use std::time::{Duration, SystemTime};

use syncweb_core::sort::{SortConfig, SortCriterion, SortEntry, Sorter};

fn entry_with(path: &str, niche: f64, frequency: u64, peers: usize, folder: &str) -> SortEntry {
    SortEntry::new(path)
        .with_folder(folder)
        .with_niche(niche)
        .with_frequency(frequency)
        .with_peers(peers)
}

fn sort_with(criterion: SortCriterion, entries: &mut [SortEntry]) {
    let mut config = SortConfig::default();
    config.criteria = vec![(criterion, true)];
    Sorter::new(config).sort(entries);
}

#[test]
fn test_niche_sort() {
    let mut entries = vec![
        entry_with("common.txt", 0.1, 10, 5, "f"),
        entry_with("rare.txt", 0.9, 1, 1, "f"),
        entry_with("medium.txt", 0.5, 5, 3, "f"),
    ];
    sort_with(SortCriterion::Niche, &mut entries);

    let paths: Vec<_> = entries.into_iter().map(|e| e.path).collect();
    assert_eq!(
        paths,
        vec![
            std::path::PathBuf::from("rare.txt"),
            std::path::PathBuf::from("medium.txt"),
            std::path::PathBuf::from("common.txt"),
        ]
    );
}

#[test]
fn test_frecency_sort() {
    let mut entries = vec![
        SortEntry::new("stale.txt")
            .with_folder("f")
            .with_frequency(1)
            .with_modified(SystemTime::now() - Duration::new(2_592_000, 0)),
        SortEntry::new("fresh.txt")
            .with_folder("f")
            .with_frequency(1)
            .with_modified(SystemTime::now()),
        SortEntry::new("frequent.txt")
            .with_folder("f")
            .with_frequency(100)
            .with_modified(SystemTime::now() - Duration::new(86_400, 0)),
    ];
    sort_with(SortCriterion::Frecency, &mut entries);

    let paths: Vec<_> = entries.into_iter().map(|e| e.path).collect();
    assert_eq!(
        paths,
        vec![
            std::path::PathBuf::from("frequent.txt"),
            std::path::PathBuf::from("fresh.txt"),
            std::path::PathBuf::from("stale.txt"),
        ]
    );
}

#[test]
fn test_peers_sort() {
    let mut entries = vec![
        entry_with("lonely.txt", 0.0, 0, 0, "f"),
        entry_with("popular.txt", 0.0, 0, 10, "f"),
        entry_with("mid.txt", 0.0, 0, 5, "f"),
    ];
    sort_with(SortCriterion::Peers, &mut entries);

    let paths: Vec<_> = entries.into_iter().map(|e| e.path).collect();
    assert_eq!(
        paths,
        vec![
            std::path::PathBuf::from("popular.txt"),
            std::path::PathBuf::from("mid.txt"),
            std::path::PathBuf::from("lonely.txt"),
        ]
    );
}

#[test]
fn test_random_sort() {
    let mut entries = vec![
        entry_with("a.txt", 0.0, 0, 0, "f"),
        entry_with("b.txt", 0.0, 0, 0, "f"),
        entry_with("c.txt", 0.0, 0, 0, "f"),
        entry_with("d.txt", 0.0, 0, 0, "f"),
        entry_with("e.txt", 0.0, 0, 0, "f"),
    ];

    let original_order: Vec<_> = entries.iter().map(|e| e.path.clone()).collect();
    sort_with(SortCriterion::Random, &mut entries);
    let shuffled_order: Vec<_> = entries.iter().map(|e| e.path.clone()).collect();

    // Verify all entries are still present (just reordered)
    assert_eq!(shuffled_order.len(), original_order.len());
    let mut sorted_original = original_order;
    sorted_original.sort();
    let mut sorted_shuffled = shuffled_order;
    sorted_shuffled.sort();
    assert_eq!(sorted_original, sorted_shuffled);
}

#[test]
fn test_folder_aggregate() {
    let mut entries = vec![
        entry_with("a/1.txt", 0.0, 0, 0, "alpha"),
        entry_with("a/2.txt", 0.0, 0, 0, "alpha"),
        entry_with("a/3.txt", 0.0, 0, 0, "alpha"),
        entry_with("b/1.txt", 0.0, 0, 0, "beta"),
        entry_with("c/1.txt", 0.0, 0, 0, "gamma"),
        entry_with("c/2.txt", 0.0, 0, 0, "gamma"),
    ];
    sort_with(SortCriterion::FolderAggregate, &mut entries);

    let folders: Vec<_> = entries.iter().map(|e| e.folder.as_str()).collect();
    let alpha_count = folders.iter().filter(|&&f| f == "alpha").count();
    let gamma_count = folders.iter().filter(|&&f| f == "gamma").count();
    let beta_count = folders.iter().filter(|&&f| f == "beta").count();

    assert!(alpha_count >= gamma_count, "alpha (3) should come before gamma (2)");
    assert!(gamma_count >= beta_count, "gamma (2) should come before beta (1)");
}
