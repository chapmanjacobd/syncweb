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

use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};

use criterion::{Criterion, criterion_group, criterion_main};
use iroh::SecretKey;
use iroh_blobs::Hash;
use syncweb_core::{
    daemon::{
        BandwidthSnapshot, DaemonHandle, DaemonState, DaemonStatus, DaemonStatusReport, FolderStatusReport, IpcClient,
        IpcCommand, IpcRequest, IpcResponse, IpcServer, ManagedPool, StateFile,
    },
    filter::{FilterAction, FilterConfig, FilterEngine, FilterEntry, FilterRule, MatchCriteria},
    folder::{CollectionEntry, CollectionManifest, DropExportOptions, DropExporter},
    indexing::{FetchFailure, FetchFailureKind, ProviderLeaseTracker, ProviderReputationStore, ReputationConfig},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
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

fn benchmark_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("benchmark runtime should build")
}

fn bench_ipc_round_trip(c: &mut Criterion) {
    let runtime = benchmark_runtime();
    let socket_path = std::env::temp_dir().join(format!("syncweb-bench-{}.sock", uuid::Uuid::new_v4()));
    let (client, server_task) = runtime.block_on(async {
        let handle = DaemonHandle::new(DaemonState::new(
            std::process::id(),
            "benchmark-node",
            1,
            std::env::temp_dir(),
            DaemonStatus::Running,
        ));
        let server = IpcServer::new(socket_path.clone(), handle);
        let task = tokio::spawn(async move { server.serve().await });
        for _ in 0..100 {
            if socket_path.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        (IpcClient::from_socket_path(socket_path.clone()), task)
    });

    {
        let mut group = c.benchmark_group("daemon_ipc");
        group.bench_function("round_trip", |b| {
            b.iter(|| {
                let response = runtime.block_on(client.send(IpcRequest::new(IpcCommand::Status)));
                let _ = std::hint::black_box(response);
            });
        });
        group.finish();
    }

    runtime.block_on(async {
        let _ = client
            .send(IpcRequest::new(IpcCommand::Shutdown { force: false }))
            .await;
        let _ = server_task.await;
    });
}

fn bench_supervisor_restart_latency(c: &mut Criterion) {
    let supervisor =
        syncweb_core::daemon::IntentSupervisor::new(3, Duration::from_millis(1), Duration::from_millis(500));
    let mut group = c.benchmark_group("daemon_supervisor");
    group.bench_function("restart_latency", |b| {
        b.iter(|| std::hint::black_box(supervisor.backoff_delay(1)));
    });
    group.finish();
}

fn bench_state_file_write_read(c: &mut Criterion) {
    let directory = std::env::temp_dir().join(format!("syncweb-state-bench-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("state benchmark directory should be created");
    let state_file = StateFile::new(&directory);
    let state = DaemonState::new(
        std::process::id(),
        "benchmark-node",
        1,
        &directory,
        DaemonStatus::Running,
    );
    let report = DaemonStatusReport::from_state(
        &state,
        2,
        vec![FolderStatusReport::new(
            "benchmark-folder",
            PathBuf::from("/tmp/benchmark-folder"),
            true,
            Some(2),
            10,
            Vec::new(),
        )],
        BandwidthSnapshot::default(),
        None,
        1,
    );

    {
        let mut group = c.benchmark_group("daemon_state");
        group.bench_function("write_read", |b| {
            b.iter(|| {
                state_file.save_status(&report).expect("state write should succeed");
                let loaded = state_file.load_status().expect("state read should succeed");
                std::hint::black_box(loaded);
            });
        });
        group.finish();
    }
    let _ = std::fs::remove_dir_all(directory);
}

struct ArchiveBenchmarkFixture {
    runtime: tokio::runtime::Runtime,
    node: Arc<IrohNode>,
    manifest: CollectionManifest,
    archive: PathBuf,
    output: PathBuf,
    pool: ManagedPool,
    directory: PathBuf,
}

fn archive_benchmark_fixture() -> ArchiveBenchmarkFixture {
    let runtime = benchmark_runtime();
    let directory = std::env::temp_dir().join(format!("syncweb-archive-bench-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("archive benchmark directory should be created");
    let (node, manifest, archive, output, pool) = runtime.block_on(async {
        let identity = IdentityManager::new(directory.join("identity.key")).expect("benchmark identity should open");
        let node = Arc::new(
            IrohNode::new(identity, directory.join("data"), RelayMode::Default)
                .await
                .expect("benchmark node should start"),
        );
        let content = b"archive benchmark content";
        let content_hash = node
            .blob_store()
            .add_bytes(content)
            .await
            .expect("benchmark content should be stored");
        let mut manifest = CollectionManifest::new(uuid::Uuid::new_v4(), "1.0.0");
        manifest.entries.push(
            CollectionEntry::new(content_hash, "benchmark.txt", content.len() as u64)
                .expect("benchmark entry should be valid"),
        );
        let archive = directory.join("input.car.zst");
        DropExporter::new(node.blob_store().clone())
            .export_drop_with_options(
                std::slice::from_ref(&manifest),
                &archive,
                DropExportOptions::default(),
                None,
            )
            .await
            .expect("benchmark archive should be exported");
        let output = directory.join("output.car.zst");
        let pool = ManagedPool::new("syncweb-benchmark", 1).expect("benchmark pool should start");
        (node, manifest, archive, output, pool)
    });
    ArchiveBenchmarkFixture {
        runtime,
        node,
        manifest,
        archive,
        output,
        pool,
        directory,
    }
}

fn bench_archive_export_with_pool(c: &mut Criterion) {
    let fixture = archive_benchmark_fixture();
    let exporter = DropExporter::new(fixture.node.blob_store().clone());
    {
        let mut group = c.benchmark_group("daemon_archive_export");
        group.bench_function("without_pool", |b| {
            b.iter(|| {
                let result = fixture.runtime.block_on(exporter.export_drop_with_options(
                    std::slice::from_ref(&fixture.manifest),
                    &fixture.output,
                    DropExportOptions::default(),
                    None,
                ));
                let _ = std::hint::black_box(result);
            });
        });
        group.bench_function("with_pool", |b| {
            b.iter(|| {
                let result = fixture.runtime.block_on(exporter.export_drop_with_options(
                    std::slice::from_ref(&fixture.manifest),
                    &fixture.output,
                    DropExportOptions::default(),
                    Some(&fixture.pool),
                ));
                let _ = std::hint::black_box(result);
            });
        });
        group.finish();
    }
    fixture
        .runtime
        .block_on(fixture.node.stop())
        .expect("benchmark node should stop");
    let _ = std::fs::remove_dir_all(fixture.directory);
}

fn bench_archive_import_with_pool(c: &mut Criterion) {
    let fixture = archive_benchmark_fixture();
    let importer = syncweb_core::DropImporter::new(fixture.node.blob_store().clone());
    {
        let mut group = c.benchmark_group("daemon_archive_import");
        group.bench_function("without_pool", |b| {
            b.iter(|| {
                let result = fixture.runtime.block_on(importer.import_archive(
                    &fixture.archive,
                    syncweb_core::DropImportOptions::default(),
                    None,
                ));
                let _ = std::hint::black_box(result);
            });
        });
        group.bench_function("with_pool", |b| {
            b.iter(|| {
                let result = fixture.runtime.block_on(importer.import_archive(
                    &fixture.archive,
                    syncweb_core::DropImportOptions::default(),
                    Some(&fixture.pool),
                ));
                let _ = std::hint::black_box(result);
            });
        });
        group.finish();
    }
    fixture
        .runtime
        .block_on(fixture.node.stop())
        .expect("benchmark node should stop");
    let _ = std::fs::remove_dir_all(fixture.directory);
}

fn bench_ipc_create_folder(c: &mut Criterion) {
    let runtime = benchmark_runtime();
    let fixture = {
        let directory = std::env::temp_dir().join(format!("syncweb-bench-create-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("benchmark directory should be created");
        let (node, handle, pool) = runtime.block_on(async {
            let identity = syncweb_core::node::identity::IdentityManager::new(directory.join("identity.key"))
                .expect("benchmark identity should open");
            let node = Arc::new(
                IrohNode::new(identity, directory.join("data"), RelayMode::Default)
                    .await
                    .expect("benchmark node should start"),
            );
            let daemon_state = DaemonState::new(
                std::process::id(),
                node.endpoint().id().to_string(),
                1,
                &directory,
                DaemonStatus::Running,
            );
            let handle = DaemonHandle::new(daemon_state);
            let pool = Arc::new(ManagedPool::new("syncweb-bench", 1).expect("benchmark pool should start"));
            (node, handle, pool)
        });
        let socket_path = std::env::temp_dir().join(format!("syncweb-bench-create-{}.sock", uuid::Uuid::new_v4()));
        let server = IpcServer::with_archive_context(socket_path.clone(), handle, node.clone(), pool);
        (node, server, socket_path, directory)
    };

    {
        let mut group = c.benchmark_group("daemon_ipc");
        let test_dir = fixture.3.join("bench-create-folder");
        group.bench_function("create_folder", |b| {
            b.iter(|| {
                let response = runtime.block_on(fixture.1.handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                    path: test_dir.clone(),
                    mode: "sendreceive".to_owned(),
                })));
                let _ = std::hint::black_box(response);
            });
        });
        group.finish();
    }

    runtime.block_on(fixture.0.stop()).expect("benchmark node should stop");
    let _ = std::fs::remove_dir_all(fixture.3);
}

fn bench_ipc_health_check(c: &mut Criterion) {
    let runtime = benchmark_runtime();
    let fixture = {
        let directory = std::env::temp_dir().join(format!("syncweb-bench-health-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("benchmark directory should be created");
        let (node, handle, pool, namespace) = runtime.block_on(async {
            let identity = syncweb_core::node::identity::IdentityManager::new(directory.join("identity.key"))
                .expect("benchmark identity should open");
            let node = Arc::new(
                IrohNode::new(identity, directory.join("data"), RelayMode::Default)
                    .await
                    .expect("benchmark node should start"),
            );
            let daemon_state = DaemonState::new(
                std::process::id(),
                node.endpoint().id().to_string(),
                1,
                &directory,
                DaemonStatus::Running,
            );
            let handle = DaemonHandle::new(daemon_state);
            let pool = Arc::new(ManagedPool::new("syncweb-bench", 1).expect("benchmark pool should start"));
            let server = IpcServer::with_archive_context(
                std::path::PathBuf::from(""),
                handle.clone(),
                node.clone(),
                pool.clone(),
            );
            let test_dir = directory.join("health-bench-folder");
            let response = server
                .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                    path: test_dir.clone(),
                    mode: "sendreceive".to_owned(),
                }))
                .await;
            let namespace = if let IpcResponse::Ok { message } = &response {
                message
                    .lines()
                    .find(|line| line.starts_with("namespace:"))
                    .and_then(|line| line.strip_prefix("namespace:").map(str::trim).map(String::from))
            } else {
                None
            };
            (node, handle, pool, namespace)
        });
        let socket_path = std::env::temp_dir().join(format!("syncweb-bench-health-{}.sock", uuid::Uuid::new_v4()));
        let server = IpcServer::with_archive_context(socket_path.clone(), handle, node.clone(), pool);
        (node, server, socket_path, directory, namespace)
    };

    if let Some(ref ns) = fixture.4 {
        let namespace = ns.clone();
        let mut group = c.benchmark_group("daemon_ipc");
        group.bench_function("health_check", |b| {
            b.iter(|| {
                let response = runtime.block_on(fixture.1.handle_request(IpcRequest::new(IpcCommand::HealthCheck {
                    path: std::path::PathBuf::from(&namespace),
                })));
                let _ = std::hint::black_box(response);
            });
        });
        group.finish();
    }

    runtime.block_on(fixture.0.stop()).expect("benchmark node should stop");
    let _ = std::fs::remove_dir_all(fixture.3);
}

fn bench_ipc_verify_integrity(c: &mut Criterion) {
    let runtime = benchmark_runtime();
    let fixture = {
        let directory = std::env::temp_dir().join(format!("syncweb-bench-verify-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("benchmark directory should be created");
        let (node, handle, pool, namespace) = runtime.block_on(async {
            let identity = syncweb_core::node::identity::IdentityManager::new(directory.join("identity.key"))
                .expect("benchmark identity should open");
            let node = Arc::new(
                IrohNode::new(identity, directory.join("data"), RelayMode::Default)
                    .await
                    .expect("benchmark node should start"),
            );
            let daemon_state = DaemonState::new(
                std::process::id(),
                node.endpoint().id().to_string(),
                1,
                &directory,
                DaemonStatus::Running,
            );
            let handle = DaemonHandle::new(daemon_state);
            let pool = Arc::new(ManagedPool::new("syncweb-bench", 1).expect("benchmark pool should start"));
            let server = IpcServer::with_archive_context(
                std::path::PathBuf::from(""),
                handle.clone(),
                node.clone(),
                pool.clone(),
            );
            let test_dir = directory.join("verify-bench-folder");
            let response = server
                .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                    path: test_dir.clone(),
                    mode: "sendreceive".to_owned(),
                }))
                .await;
            let namespace = if let IpcResponse::Ok { message } = &response {
                message
                    .lines()
                    .find(|line| line.starts_with("namespace:"))
                    .and_then(|line| line.strip_prefix("namespace:").map(str::trim).map(String::from))
            } else {
                None
            };
            (node, handle, pool, namespace)
        });
        let socket_path = std::env::temp_dir().join(format!("syncweb-bench-verify-{}.sock", uuid::Uuid::new_v4()));
        let server = IpcServer::with_archive_context(socket_path.clone(), handle, node.clone(), pool);
        (node, server, socket_path, directory, namespace)
    };

    if let Some(ref ns) = fixture.4 {
        let namespace = ns.clone();
        let mut group = c.benchmark_group("daemon_ipc");
        group.bench_function("verify_integrity", |b| {
            b.iter(|| {
                let response =
                    runtime.block_on(fixture.1.handle_request(IpcRequest::new(IpcCommand::VerifyIntegrity {
                        path: std::path::PathBuf::from(&namespace),
                    })));
                let _ = std::hint::black_box(response);
            });
        });
        group.finish();
    }

    runtime.block_on(fixture.0.stop()).expect("benchmark node should stop");
    let _ = std::fs::remove_dir_all(fixture.3);
}

fn bench_daemon_start_stop(c: &mut Criterion) {
    let runtime = benchmark_runtime();
    let directory = std::env::temp_dir().join(format!("syncweb-bench-startstop-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("benchmark directory should be created");

    {
        let mut group = c.benchmark_group("daemon_lifecycle");
        group.bench_function("start_stop", |b| {
            b.iter(|| {
                let result = runtime.block_on(async {
                    let identity = syncweb_core::node::identity::IdentityManager::new(directory.join("identity.key"))
                        .expect("benchmark identity should open");
                    let node = IrohNode::new(identity, directory.join("data"), RelayMode::Default)
                        .await
                        .expect("benchmark node should start");
                    node.stop().await
                });
                let _ = std::hint::black_box(result);
            });
        });
        group.finish();
    }

    let _ = std::fs::remove_dir_all(directory);
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
    bench_ipc_round_trip,
    bench_supervisor_restart_latency,
    bench_state_file_write_read,
    bench_archive_export_with_pool,
    bench_archive_import_with_pool,
    bench_ipc_create_folder,
    bench_ipc_health_check,
    bench_ipc_verify_integrity,
    bench_daemon_start_stop,
);
criterion_main!(benches);
