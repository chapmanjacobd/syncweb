use std::collections::HashMap;

use syncweb_core::sort::{SortConfig, SortCriterion, SortEntry, Sorter};

#[test]
fn test_sort_with_peer_enrichment() {
    let mut entries = vec![
        SortEntry::new("a.txt").with_folder("f").with_size(100),
        SortEntry::new("b.txt").with_folder("f").with_size(200),
    ];
    let enrichment: HashMap<String, usize> = [("a.txt".to_string(), 5), ("b.txt".to_string(), 1)]
        .into_iter()
        .collect();
    let sorter = Sorter::new(SortConfig::default());
    sorter.enrich_peers(&mut entries, &enrichment);
    assert_eq!(entries.first().map(|e| e.peers), Some(5));
    assert_eq!(entries.get(1).map(|e| e.peers), Some(1));
}

#[test]
fn test_sort_by_peers_with_enrichment() {
    let mut entries = vec![
        SortEntry::new("a.txt").with_folder("f"),
        SortEntry::new("b.txt").with_folder("f"),
    ];
    let enrichment: HashMap<String, usize> = [("a.txt".to_string(), 1), ("b.txt".to_string(), 10)]
        .into_iter()
        .collect();
    let mut config = SortConfig::default();
    config.criteria = vec![(SortCriterion::Peers, true)];
    let sorter = Sorter::new(config);
    sorter.enrich_peers(&mut entries, &enrichment);
    sorter.sort(&mut entries);
    assert_eq!(
        entries.first().map(|e| e.path.clone()),
        Some(std::path::PathBuf::from("b.txt"))
    );
    assert_eq!(
        entries.get(1).map(|e| e.path.clone()),
        Some(std::path::PathBuf::from("a.txt"))
    );
}

#[test]
fn test_sort_by_niche_with_enrichment() {
    let mut entries = vec![
        SortEntry::new("popular.txt").with_folder("f"),
        SortEntry::new("rare.txt").with_folder("f"),
    ];
    let enrichment: HashMap<String, usize> = [("popular.txt".to_string(), 100), ("rare.txt".to_string(), 2)]
        .into_iter()
        .collect();
    let mut config = SortConfig::default();
    config.criteria = vec![(SortCriterion::Niche, true)];
    config.niche = 3;
    let sorter = Sorter::new(config);
    sorter.enrich_peers(&mut entries, &enrichment);
    sorter.enrich_niche(&mut entries);
    sorter.sort(&mut entries);
    assert_eq!(
        entries.first().map(|e| e.path.clone()),
        Some(std::path::PathBuf::from("rare.txt"))
    );
    assert_eq!(
        entries.get(1).map(|e| e.path.clone()),
        Some(std::path::PathBuf::from("popular.txt"))
    );
}

#[test]
fn test_enrich_frequency() {
    let mut entries = vec![SortEntry::new("a.txt"), SortEntry::new("b.txt")];
    let freq_map: HashMap<String, u64> = [("a.txt".to_string(), 42), ("b.txt".to_string(), 7)]
        .into_iter()
        .collect();
    let sorter = Sorter::new(SortConfig::default());
    sorter.enrich_frequency(&mut entries, &freq_map);
    assert_eq!(entries.first().map(|e| e.frequency), Some(42));
    assert_eq!(entries.get(1).map(|e| e.frequency), Some(7));
}

#[test]
fn test_enrich_missing_path_defaults_to_zero() {
    let mut entries = vec![SortEntry::new("unknown.txt")];
    let enrichment: HashMap<String, usize> = HashMap::new();
    let sorter = Sorter::new(SortConfig::default());
    sorter.enrich_peers(&mut entries, &enrichment);
    assert_eq!(entries.first().map(|e| e.peers), Some(0));
}
