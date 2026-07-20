use semver::Version;
use std::time::Duration;
use syncweb_core::filter::{FilterAction, FilterConfig, FilterEngine, FilterEntry, FilterRule, MatchCriteria};

#[test]
fn filter_engine_applies_rules_versions_and_folder_overrides() -> anyhow::Result<()> {
    let mut temporary = MatchCriteria::default();
    temporary.name = Some("*.tmp".to_owned());
    let mut modern = MatchCriteria::default();
    modern.version = Some(">=2.0.0".to_owned());
    let mut private = MatchCriteria::default();
    private.path = Some("private/**".to_owned());

    let mut config = FilterConfig::default();
    config.rules.push(FilterRule::new(FilterAction::Reject, temporary));
    config.rules.push(FilterRule::new(FilterAction::Accept, modern));
    config
        .folders
        .insert("work".to_owned(), vec![FilterRule::new(FilterAction::Reject, private)]);
    let engine = FilterEngine::new(config)?;

    anyhow::ensure!(engine.evaluate(&FilterEntry::new("cache.tmp", 10)) == FilterAction::Reject);
    anyhow::ensure!(
        engine.evaluate(&FilterEntry::new("release.bin", 10).with_version(Version::parse("2.1.0")?))
            == FilterAction::Accept
    );
    anyhow::ensure!(
        engine.evaluate_for_folder("work", &FilterEntry::new("private/key.txt", 10)) == FilterAction::Reject
    );
    Ok(())
}

#[test]
fn test_filter_eval_perf() -> anyhow::Result<()> {
    let mut config = FilterConfig::default();
    for i in 0..100 {
        let mut criteria = MatchCriteria::default();
        criteria.name = Some(format!("pattern-{i}.*"));
        config.rules.push(FilterRule::new(FilterAction::Reject, criteria));
    }
    let engine = FilterEngine::new(config)?;

    let entries: Vec<FilterEntry> = (0_u64..10_000_u64)
        .map(|i| FilterEntry::new(format!("file-{i}.dat"), i))
        .collect();

    let start = std::time::Instant::now();
    for entry in &entries {
        let _ = engine.evaluate(entry);
    }
    let elapsed = start.elapsed();

    anyhow::ensure!(
        elapsed < Duration::from_secs(2),
        "10k filter evaluations took {elapsed:?}, expected < 2s in debug"
    );
    Ok(())
}

#[test]
fn test_filter_global_rules_apply_without_folder() -> anyhow::Result<()> {
    let mut criteria = MatchCriteria::default();
    criteria.extensions = Some(vec![".log".to_owned()]);
    let mut config = FilterConfig::default();
    config.rules.push(FilterRule::new(FilterAction::Reject, criteria));

    let engine = FilterEngine::new(config)?;
    anyhow::ensure!(engine.evaluate(&FilterEntry::new("app.log", 100)) == FilterAction::Reject);
    anyhow::ensure!(engine.evaluate(&FilterEntry::new("readme.md", 100)) == FilterAction::Accept);
    Ok(())
}

#[test]
fn test_filter_folder_rules_take_precedence() -> anyhow::Result<()> {
    let mut global_criteria = MatchCriteria::default();
    global_criteria.name = Some("*.log".to_owned());
    let mut folder_criteria = MatchCriteria::default();
    folder_criteria.name = Some("important.log".to_owned());

    let mut config = FilterConfig::default();
    config
        .rules
        .push(FilterRule::new(FilterAction::Reject, global_criteria));
    config.folders.insert(
        "special".to_owned(),
        vec![FilterRule::new(FilterAction::Accept, folder_criteria)],
    );

    let engine = FilterEngine::new(config)?;

    // Globally rejected, but special folder overrides to accept.
    let entry = FilterEntry::new("important.log", 10);
    anyhow::ensure!(engine.evaluate(&entry) == FilterAction::Reject);
    anyhow::ensure!(engine.evaluate_for_folder("special", &entry) == FilterAction::Accept);
    Ok(())
}

#[test]
fn test_filter_size_criteria() -> anyhow::Result<()> {
    let mut criteria = MatchCriteria::default();
    criteria.min_size = Some(100);
    criteria.max_size = Some(500);
    let mut config = FilterConfig::default();
    config.rules.push(FilterRule::new(FilterAction::Reject, criteria));

    let engine = FilterEngine::new(config)?;
    anyhow::ensure!(engine.evaluate(&FilterEntry::new("tiny.txt", 10)) == FilterAction::Accept);
    anyhow::ensure!(engine.evaluate(&FilterEntry::new("mid.txt", 200)) == FilterAction::Reject);
    anyhow::ensure!(engine.evaluate(&FilterEntry::new("huge.txt", 1000)) == FilterAction::Accept);
    Ok(())
}
