use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use globset::{Glob, GlobMatcher};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize, ser::SerializeStruct};

use crate::{
    error::{Result, SyncwebError},
    fs::FileEntry,
};

/// Result of evaluating a filter rule.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FilterAction {
    Accept,
    Reject,
}

/// Conditions in one filter rule. Populated conditions are combined with AND.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[non_exhaustive]
pub struct MatchCriteria {
    pub name: Option<String>,
    #[serde(alias = "ext")]
    pub extensions: Option<Vec<String>>,
    pub path: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_duration")]
    pub age: Option<Duration>,
    pub min_seeders: Option<usize>,
    pub version: Option<String>,
}

impl Serialize for MatchCriteria {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("MatchCriteria", 8)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("extensions", &self.extensions)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("min_size", &self.min_size)?;
        state.serialize_field("max_size", &self.max_size)?;
        state.serialize_field("age", &self.age.map(|duration| duration.as_secs()))?;
        state.serialize_field("min_seeders", &self.min_seeders)?;
        state.serialize_field("version", &self.version)?;
        state.end()
    }
}

/// An ordered accept or reject rule.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FilterRule {
    #[serde(alias = "type")]
    pub action: FilterAction,
    #[serde(rename = "match")]
    pub criteria: MatchCriteria,
}

impl FilterRule {
    #[must_use]
    pub const fn new(action: FilterAction, criteria: MatchCriteria) -> Self {
        Self { action, criteria }
    }
}

/// Entry metadata consumed by the filter evaluator.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FilterEntry {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub seeders: usize,
    pub version: Option<Version>,
}

impl FilterEntry {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, size: u64) -> Self {
        Self {
            path: path.into(),
            size,
            modified: SystemTime::now(),
            seeders: 0,
            version: None,
        }
    }

    #[must_use]
    pub const fn with_modified(mut self, modified: SystemTime) -> Self {
        self.modified = modified;
        self
    }

    #[must_use]
    pub const fn with_seeders(mut self, seeders: usize) -> Self {
        self.seeders = seeders;
        self
    }

    #[must_use]
    pub fn with_version(mut self, version: Version) -> Self {
        self.version = Some(version);
        self
    }

    #[must_use]
    pub fn from_file(entry: &FileEntry) -> Self {
        Self {
            path: entry.relative_path.clone(),
            size: entry.size,
            modified: entry.modified,
            seeders: 0,
            version: None,
        }
    }
}

/// Serializable global and per-folder filter configuration.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FilterConfig {
    #[serde(default)]
    pub rules: Vec<FilterRule>,
    #[serde(default)]
    pub folders: HashMap<String, Vec<FilterRule>>,
}

#[derive(Clone, Debug)]
struct CompiledRule {
    rule: FilterRule,
    name: Option<GlobMatcher>,
    path: Option<GlobMatcher>,
    version: Option<VersionReq>,
}

/// Ordered rules-based evaluator used by automatic synchronization.
#[derive(Clone, Debug, Default)]
pub struct FilterEngine {
    global: Vec<CompiledRule>,
    folders: HashMap<String, Vec<CompiledRule>>,
}

impl FilterEngine {
    /// Compile a filter configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when a glob or semantic-version requirement is invalid.
    pub fn new(config: FilterConfig) -> Result<Self> {
        let global = compile_rules(config.rules)?;
        let folders = config
            .folders
            .into_iter()
            .map(|(folder, rules)| compile_rules(rules).map(|compiled| (folder, compiled)))
            .collect::<Result<HashMap<_, _>>>()?;
        Ok(Self { global, folders })
    }

    /// Read and compile a TOML filter file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, parsed, or compiled.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|error| SyncwebError::operation("failed to read filter configuration", error))?;
        let config = toml::from_str(&contents)
            .map_err(|error| SyncwebError::operation("failed to parse filter configuration", error))?;
        Self::new(config)
    }

    /// Evaluate global rules. The first matching rule wins; unmatched entries
    /// are accepted.
    #[must_use]
    pub fn evaluate(&self, entry: &FilterEntry) -> FilterAction {
        evaluate_rules(&self.global, entry).unwrap_or(FilterAction::Accept)
    }

    /// Evaluate per-folder rules before global rules.
    #[must_use]
    pub fn evaluate_for_folder(&self, folder: &str, entry: &FilterEntry) -> FilterAction {
        self.folders
            .get(folder)
            .and_then(|rules| evaluate_rules(rules, entry))
            .unwrap_or_else(|| self.evaluate(entry))
    }

    #[must_use]
    pub fn filter_for_folder<'a>(
        &self,
        folder: &str,
        entries: impl IntoIterator<Item = &'a FilterEntry>,
    ) -> Vec<&'a FilterEntry> {
        entries
            .into_iter()
            .filter(|entry| self.evaluate_for_folder(folder, entry) == FilterAction::Accept)
            .collect()
    }

    #[must_use]
    pub fn filter<'a>(&self, entries: impl IntoIterator<Item = &'a FilterEntry>) -> Vec<&'a FilterEntry> {
        entries
            .into_iter()
            .filter(|entry| self.evaluate(entry) == FilterAction::Accept)
            .collect()
    }

    #[must_use]
    pub fn config(&self) -> FilterConfig {
        FilterConfig {
            rules: self.global.iter().map(|compiled| compiled.rule.clone()).collect(),
            folders: self
                .folders
                .iter()
                .map(|(folder, rules)| {
                    (
                        folder.clone(),
                        rules.iter().map(|compiled| compiled.rule.clone()).collect(),
                    )
                })
                .collect(),
        }
    }
}

fn compile_rules(rules: Vec<FilterRule>) -> Result<Vec<CompiledRule>> {
    rules
        .into_iter()
        .map(|rule| {
            let name = compile_glob(rule.criteria.name.as_deref())?;
            let path = compile_glob(rule.criteria.path.as_deref())?;
            let version = rule
                .criteria
                .version
                .as_deref()
                .map(VersionReq::parse)
                .transpose()
                .map_err(|error| SyncwebError::InvalidConfig(format!("invalid version requirement: {error}")))?;
            Ok(CompiledRule {
                rule,
                name,
                path,
                version,
            })
        })
        .collect()
}

fn compile_glob(pattern: Option<&str>) -> Result<Option<GlobMatcher>> {
    pattern
        .map(|value| {
            Glob::new(value)
                .map(|glob| glob.compile_matcher())
                .map_err(|error| SyncwebError::InvalidConfig(format!("invalid filter glob {value:?}: {error}")))
        })
        .transpose()
}

fn evaluate_rules(rules: &[CompiledRule], entry: &FilterEntry) -> Option<FilterAction> {
    rules
        .iter()
        .find(|compiled| matches_rule(compiled, entry))
        .map(|compiled| compiled.rule.action)
}

fn matches_rule(compiled: &CompiledRule, entry: &FilterEntry) -> bool {
    let criteria = &compiled.rule.criteria;
    if compiled.name.as_ref().is_some_and(|matcher| {
        entry
            .path
            .file_name()
            .is_none_or(|name| !matcher.is_match(Path::new(name)))
    }) {
        return false;
    }
    if compiled
        .path
        .as_ref()
        .is_some_and(|matcher| !matcher.is_match(&entry.path))
    {
        return false;
    }
    if criteria.extensions.as_ref().is_some_and(|extensions| {
        entry
            .path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| {
                !extensions
                    .iter()
                    .any(|expected| expected.trim_start_matches('.').eq_ignore_ascii_case(extension))
            })
    }) {
        return false;
    }
    if criteria.min_size.is_some_and(|minimum| entry.size < minimum)
        || criteria.max_size.is_some_and(|maximum| entry.size > maximum)
        || criteria.min_seeders.is_some_and(|minimum| entry.seeders < minimum)
    {
        return false;
    }
    if criteria.age.is_some_and(|minimum_age| {
        SystemTime::now()
            .duration_since(entry.modified)
            .is_ok_and(|age| age < minimum_age)
    }) {
        return false;
    }
    if compiled.version.as_ref().is_some_and(|requirement| {
        entry
            .version
            .as_ref()
            .is_none_or(|version| !requirement.matches(version))
    }) {
        return false;
    }
    true
}

fn deserialize_optional_duration<'de, D>(deserializer: D) -> std::result::Result<Option<Duration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DurationValue {
        Seconds(u64),
        Text(String),
    }

    fn parse(value: DurationValue) -> std::result::Result<Duration, String> {
        match value {
            DurationValue::Seconds(seconds) => Ok(Duration::from_secs(seconds)),
            DurationValue::Text(text) => {
                let trimmed = text.trim();
                let unit_start = trimmed
                    .char_indices()
                    .find_map(|(index, character)| character.is_ascii_alphabetic().then_some(index))
                    .unwrap_or(trimmed.len());
                let (number, unit) = trimmed.split_at(unit_start);
                let amount = number
                    .parse::<u64>()
                    .map_err(|error| format!("invalid duration {trimmed:?}: {error}"))?;
                let multiplier = match unit {
                    "s" | "" => 1,
                    "m" => 60,
                    "h" => 60 * 60,
                    "d" => 24 * 60 * 60,
                    "w" => 7 * 24 * 60 * 60,
                    _ => return Err(format!("unsupported duration unit {unit:?}")),
                };
                amount
                    .checked_mul(multiplier)
                    .map(Duration::from_secs)
                    .ok_or_else(|| format!("duration {trimmed:?} is too large"))
            }
        }
    }

    Option::<DurationValue>::deserialize(deserializer)?
        .map(parse)
        .transpose()
        .map_err(serde::de::Error::custom)
}
