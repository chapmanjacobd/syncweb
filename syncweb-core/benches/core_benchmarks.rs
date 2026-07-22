#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::float_arithmetic,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::as_conversions,
    clippy::assigning_clones,
    clippy::option_if_let_else,
    clippy::manual_is_variant_and,
    clippy::unreadable_literal,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::wildcard_imports,
    clippy::must_use_candidate,
    clippy::use_debug,
    clippy::similar_names,
    clippy::items_after_statements,
    clippy::unseparated_literal_suffix
)]

use std::time::{Duration, SystemTime};

use criterion::{Criterion, criterion_group, criterion_main};
use iroh::SecretKey;
use iroh_blobs::Hash;
use syncweb_core::{
    filter::{FilterAction, FilterConfig, FilterEngine, FilterEntry, FilterRule, MatchCriteria},
    indexing::{FetchFailure, FetchFailureKind, ProviderLeaseTracker, ProviderReputationStore, ReputationConfig},
    schedule::{BandwidthWindowConfig, ScheduleConfig, ScheduleFolderConfig, ScheduleManager},
    search::{FindEngine, FindQuery},
    sort::{SortConfig, SortCriterion, SortEntry, Sorter},
    stats::BandwidthStats,
};

fn make_criteria(extensions: Option<Vec<String>>, min_seeders: Option<usize>, max_size: Option<u64>) -> MatchCriteria {
    let mut c = MatchCriteria::default();
    c.extensions = extensions;
    c.min_seeders = min_seeders;
    c.max_size = max_size;
    c
}

fn bench_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter");

    let mut config = FilterConfig::default();
    config.rules = vec![
        FilterRule::new(
            FilterAction::Reject,
            make_criteria(Some(vec![String::from("tmp")]), None, None),
        ),
        FilterRule::new(FilterAction::Reject, make_criteria(None, Some(10), None)),
        FilterRule::new(FilterAction::Accept, make_criteria(None, None, Some(1_000_000_000))),
    ];

    let Ok(engine) = FilterEngine::new(config) else {
        unreachable!();
    };
    let entries: Vec<FilterEntry> = (0_usize..1_000)
        .map(|i: usize| {
            FilterEntry::new(
                format!("file-{i}.txt"),
                u64::try_from(i.checked_mul(1000).unwrap_or(0)).unwrap_or_default(),
            )
            .with_seeders(i.checked_rem(15).unwrap_or(0))
        })
        .collect();

    group.bench_function("evaluate_1k", |b| {
        b.iter(|| {
            for entry in &entries {
                let _ = std::hint::black_box(engine.evaluate(entry));
            }
        });
    });

    group.bench_function("filter_1k", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(engine.filter(entries.iter()));
        });
    });

    group.finish();
}

fn bench_filter_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_compile");

    group.bench_function("compile_10_rules", |b| {
        b.iter(|| {
            let rules: Vec<FilterRule> = (0..10)
                .map(|i| {
                    let mut criteria = MatchCriteria::default();
                    criteria.name = Some(format!("*.ext{i}"));
                    FilterRule::new(FilterAction::Accept, criteria)
                })
                .collect();
            let mut config = FilterConfig::default();
            config.rules = rules;
            let _ = std::hint::black_box(FilterEngine::new(config));
        });
    });

    group.finish();
}

fn bench_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(20));

    let make_entries = |n: usize| -> Vec<SortEntry> {
        (0_usize..n)
            .map(|i: usize| {
                SortEntry::new(format!("file-{i}.txt"))
                    .with_folder(format!("folder-{}", i.checked_rem(10).unwrap_or(0)))
                    .with_niche(f64::from(u32::try_from(i.checked_rem(100).unwrap_or(0)).unwrap_or(0)))
                    .with_frequency(u64::try_from(i.checked_rem(50).unwrap_or(0)).unwrap_or_default())
                    .with_modified(
                        SystemTime::UNIX_EPOCH
                            .checked_add(Duration::from_secs(
                                u64::try_from(i.checked_mul(3600).unwrap_or(0)).unwrap_or_default(),
                            ))
                            .unwrap_or(SystemTime::UNIX_EPOCH),
                    )
                    .with_peers(i.checked_rem(20).unwrap_or(0))
            })
            .collect()
    };

    for (name, size) in [("small", 100), ("medium", 1_000), ("large", 10_000)] {
        let entries_base = make_entries(size);
        for criterion_variant in [
            SortCriterion::Niche,
            SortCriterion::Frecency,
            SortCriterion::Peers,
            SortCriterion::Random,
            SortCriterion::FolderAggregate,
        ] {
            let label = format!("{name}_{criterion_variant:?}").to_lowercase();
            group.bench_function(&label, |b| {
                b.iter_batched(
                    || entries_base.clone(),
                    |mut entries| {
                        let mut config = SortConfig::default();
                        config.criteria = vec![(criterion_variant, false)];
                        let sorter = Sorter::new(config);
                        sorter.sort(&mut entries);
                        std::hint::black_box(&entries);
                    },
                    criterion::BatchSize::SmallInput,
                );
            });
        }
    }

    group.finish();
}

fn bench_schedule(c: &mut Criterion) {
    let mut group = c.benchmark_group("schedule");

    let mut folders = std::collections::BTreeMap::new();
    let mut media = ScheduleFolderConfig::default();
    media.active_hours = Some(String::from("01:00-05:00"));
    media.max_download = Some(String::from("50MB/s"));
    folders.insert(String::from("media"), media);
    let mut backup = ScheduleFolderConfig::default();
    backup.active_hours = Some(String::from("02:00-06:00"));
    backup.max_upload = Some(String::from("20MB/s"));
    folders.insert(String::from("backup"), backup);
    let mut config = ScheduleConfig::default();
    config.active_hours = String::from("22:00-06:00");
    config.bandwidth = vec![
        BandwidthWindowConfig::new("08:00-18:00", "1MB/s", "5MB/s"),
        BandwidthWindowConfig::new("18:00-08:00", "0", "0"),
    ];
    config.folders = folders;
    let Ok(manager) = ScheduleManager::from_config(&config) else {
        unreachable!();
    };

    group.bench_function("is_active_at", |b| {
        b.iter(|| {
            for minute in (0..1440).step_by(10) {
                let _ = std::hint::black_box(manager.is_active_at(None, minute));
            }
        });
    });

    group.bench_function("current_limits_at", |b| {
        b.iter(|| {
            for minute in (0..1440).step_by(10) {
                let _ = std::hint::black_box(manager.current_limits_at(None, minute));
            }
        });
    });

    group.bench_function("per_folder_evaluate", |b| {
        b.iter(|| {
            for minute in (0..1440).step_by(10) {
                let _ = std::hint::black_box(manager.is_active_at(Some("media"), minute));
                let _ = std::hint::black_box(manager.current_limits_at(Some("backup"), minute));
            }
        });
    });

    group.finish();
}

fn bench_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats");

    group.bench_function("record_download_1k", |b| {
        b.iter_batched(
            BandwidthStats::default,
            |mut stats| {
                for i in 0usize..1_000 {
                    stats.record_download(
                        u64::try_from(i.checked_mul(100).unwrap_or(0)).unwrap_or_default(),
                        1,
                        Some("folder"),
                        Some("peer"),
                    );
                }
                std::hint::black_box(&stats);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("record_upload_1k", |b| {
        b.iter_batched(
            BandwidthStats::default,
            |mut stats| {
                for i in 0usize..1_000 {
                    stats.record_upload(
                        u64::try_from(i.checked_mul(100).unwrap_or(0)).unwrap_or_default(),
                        1,
                        Some("folder"),
                        Some("peer"),
                    );
                }
                std::hint::black_box(&stats);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    let entries: Vec<syncweb_core::fs::FileEntry> = (0_usize..1_000)
        .map(|i: usize| {
            let path = std::path::PathBuf::from(format!("/tmp/bench/file-{i}.txt"));
            let entry = syncweb_core::fs::FileEntry::builder()
                .path(path.clone())
                .relative_path(path)
                .size(u64::try_from(i.checked_mul(100).unwrap_or(0)).unwrap_or_default())
                .modified(SystemTime::UNIX_EPOCH)
                .hash(blake3::Hash::from([0_u8; 32]))
                .file_type(syncweb_core::fs::FileType::File)
                .build();
            let Ok(v) = entry else {
                unreachable!();
            };
            v
        })
        .collect();

    let engine = FindEngine::new("/tmp/bench");
    let queries = [
        ("glob", FindQuery::glob("*.txt")),
        ("exact", FindQuery::exact("file-500.txt")),
        ("regex", FindQuery::regex(r"file-\d+\.txt")),
    ];

    for (kind, query) in queries {
        group.bench_function(kind, |b| {
            b.iter(|| {
                let _ = std::hint::black_box(engine.filter(&entries, &query));
            });
        });
    }

    group.finish();
}

fn bench_failure_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("phase9_failure_tracking");

    group.bench_function("track_1000_hashes", |b| {
        b.iter_batched(
            ProviderLeaseTracker::default,
            |mut tracker| {
                for hash_idx in 0u8..255 {
                    let hash = Hash::from_bytes([hash_idx; 32]);
                    for provider_idx in 0u8..4 {
                        let key = SecretKey::from_bytes(&[provider_idx; 32]).public();
                        let failure = FetchFailure::new_at(FetchFailureKind::NotFound, key, hash, 100, "missing");
                        tracker.record_failure_at(hash, key, failure, 100);
                    }
                }
                std::hint::black_box(&tracker);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("ban_lookup", |b| {
        b.iter_batched(
            || {
                let mut tracker = ProviderLeaseTracker::default();
                for i in 0u8..100 {
                    let key = SecretKey::from_bytes(&[i; 32]).public();
                    let hash = Hash::from_bytes([i; 32]);
                    tracker.ban_provider(
                        key,
                        Some(hash),
                        "bench ban",
                        syncweb_core::indexing::BanSource::Automated,
                        Some(Duration::from_hours(1)),
                        100,
                    );
                }
                tracker
            },
            |tracker| {
                for i in 0u8..100 {
                    let key = SecretKey::from_bytes(&[i; 32]).public();
                    let hash = Hash::from_bytes([i; 32]);
                    let _ = std::hint::black_box(tracker.is_banned(key, &hash, 101));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("retroactive_invalidate_100", |b| {
        b.iter_batched(
            || {
                let mut tracker = ProviderLeaseTracker::default();
                for i in 0u8..100 {
                    let key = SecretKey::from_bytes(&[i; 32]).public();
                    let hash = Hash::from_bytes([42; 32]);
                    let failure = FetchFailure::new_at(FetchFailureKind::NotFound, key, hash, 100, "missing");
                    tracker.record_failure_at(hash, key, failure, 100);
                }
                tracker
            },
            |mut tracker| {
                let hash = Hash::from_bytes([42; 32]);
                let winner = SecretKey::from_bytes(&[255; 32]).public();
                let _ = std::hint::black_box(tracker.retroactive_invalidate(hash, winner, 200));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_reputation(c: &mut Criterion) {
    let mut group = c.benchmark_group("phase9_reputation");

    group.bench_function("score_calculation", |b| {
        let mut config = ReputationConfig::default();
        config.min_samples = 1;
        let mut store = ProviderReputationStore::new(config);
        for i in 0u8..50 {
            let key = SecretKey::from_bytes(&[i; 32]).public();
            store.record_success(key, 10);
            store.record_failure(key, FetchFailureKind::Timeout, 11);
        }
        let keys: Vec<_> = (0u8..50).map(|i| SecretKey::from_bytes(&[i; 32]).public()).collect();
        b.iter(|| {
            for key in &keys {
                let _ = std::hint::black_box(store.score(*key, 20));
            }
        });
    });

    group.bench_function("rank_1000_providers", |b| {
        let mut config = ReputationConfig::default();
        config.min_samples = 1;
        let mut store = ProviderReputationStore::new(config);
        let hash = Hash::from_bytes([42; 32]);
        let keys: Vec<_> = (0u16..1000)
            .map(|i| {
                let mut seed = [0_u8; 32];
                seed[0] = (i >> 8) as u8;
                seed[1] = (i & 0xFF) as u8;
                let key = SecretKey::from_bytes(&seed).public();
                store.record_success(key, 10);
                key
            })
            .collect();
        b.iter(|| {
            let _ = std::hint::black_box(store.rank_provider_list(20, hash, &keys));
        });
    });

    group.bench_function("signal_verification", |b| {
        let mut config = ReputationConfig::default();
        config.min_samples = 1;
        let _store = ProviderReputationStore::new(config);
        let reporter_key = ed25519_dalek::SigningKey::from_bytes(&[99; 32]);
        let provider = SecretKey::from_bytes(&[1; 32]).public();
        let signal = syncweb_core::indexing::ProviderTrustSignal::new_with_time(
            provider,
            syncweb_core::indexing::TrustSignalKind::ObservedSuccess,
            None,
            1,
            &reporter_key,
        )
        .unwrap();
        b.iter(|| {
            let _ = std::hint::black_box(signal.verify_at(100));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_filter,
    bench_filter_compile,
    bench_sort,
    bench_schedule,
    bench_stats,
    bench_search,
    bench_failure_tracking,
    bench_reputation,
);
criterion_main!(benches);
