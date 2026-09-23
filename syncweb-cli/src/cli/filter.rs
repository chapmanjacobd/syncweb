use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use iroh_blobs::Hash;

use syncweb_core::{fs::FileType, parsing::parse_depth_constraints, search::FindQuery, verify::VerifyFilter};

// One content-filter vocabulary shared by `ls`, `find`, `sort`, `download`,
// and `verify`. Every command reads the same `--ext`/`--size`/`--depth`/
// `--type`/`--modified-*` dialect; the old per-command spellings survive as
// hidden aliases (`--extension`, `--sizes`, `-S`, `--levels`, `--changed-*`).
//
// `--remote-only` lives here as well so a folder download (or any folder
// command) can narrow to rows that are not yet local.
#[derive(Debug, Args, Clone, Default)]
pub struct ContentFilterArgs {
    #[arg(long, help = "Show only entries not yet downloaded (State == remote)")]
    pub remote_only: bool,

    #[arg(
        short = 'e',
        long = "ext",
        alias = "extension",
        alias = "exts",
        alias = "extensions",
        action = clap::ArgAction::Append,
        help = "File extensions to include (can repeat)"
    )]
    pub ext: Vec<String>,

    #[arg(
        long = "size",
        alias = "sizes",
        alias = "S",
        action = clap::ArgAction::Append,
        help = "Size constraints: N, -N, +N, N%10, +5GB, etc. (can repeat)"
    )]
    pub size: Vec<String>,

    #[arg(
        long = "depth",
        alias = "levels",
        alias = "d",
        action = clap::ArgAction::Append,
        help = "Depth constraints: N, +N (min), -N (max) (can repeat)"
    )]
    pub depth: Vec<String>,
    #[arg(long = "min-depth", help = "Alternative min depth notation")]
    pub min_depth: Option<usize>,
    #[arg(long = "max-depth", help = "Alternative max depth notation")]
    pub max_depth: Option<usize>,

    #[arg(
        long = "type",
        value_parser = ["f", "d", "l"],
        help = "Filter by type: f=file, d=dir, l=symlink"
    )]
    pub file_type: Option<String>,

    #[arg(
        long = "modified-within",
        alias = "changed-within",
        action = clap::ArgAction::Append,
        help = "Newer than: '3 days', '2 weeks' (can repeat)"
    )]
    pub modified_within: Vec<String>,

    #[arg(
        long = "modified-before",
        alias = "changed-before",
        action = clap::ArgAction::Append,
        help = "Older than: '3 years', '1 month' (can repeat)"
    )]
    pub modified_before: Vec<String>,

    #[arg(
        long = "time-modified",
        action = clap::ArgAction::Append,
        help = "Time modified: '-3 days' (newer), '+3 days' (older) (can repeat)"
    )]
    pub time_modified: Vec<String>,
}

impl ContentFilterArgs {
    /// True when no predicate of the group is set.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        !self.remote_only
            && self.ext.is_empty()
            && self.size.is_empty()
            && self.depth.is_empty()
            && self.min_depth.is_none()
            && self.max_depth.is_none()
            && self.file_type.is_none()
            && self.modified_within.is_empty()
            && self.modified_before.is_empty()
            && self.time_modified.is_empty()
    }

    /// Apply this group's predicates onto an existing query, leaving match
    /// semantics (`pattern`, `kind`, case, hidden, path mode) untouched.
    ///
    /// # Errors
    ///
    /// Returns an error when a size or time constraint cannot be parsed.
    pub fn apply_to(&self, query: &mut FindQuery) -> Result<()> {
        let (min_depth, max_depth) = parse_depth_constraints(&self.depth, self.min_depth.unwrap_or(0), self.max_depth);
        query.min_depth = Some(min_depth);
        query.max_depth = max_depth;

        let (min_size, max_size) = FindQuery::parse_size_constraints(&self.size)?;
        query.min_size = min_size;
        query.max_size = max_size;

        let (after, before) =
            FindQuery::parse_time_constraints(&self.modified_within, &self.modified_before, &self.time_modified)?;
        query.modified_after = after;
        query.modified_before = before;

        if !self.ext.is_empty() {
            query.extensions.clone_from(&self.ext);
        }

        query.file_type = self.file_type.clone().map(|kind| match kind.as_str() {
            "d" => FileType::Directory,
            "l" => FileType::Symlink,
            _ => FileType::File,
        });
        Ok(())
    }

    /// Build a predicate-only query: matches every entry unless a field of the
    /// group restricts it. Keeps hidden entries visible (no match semantics).
    ///
    /// # Errors
    ///
    /// Returns an error when a size or time constraint cannot be parsed.
    pub fn to_find_query(&self) -> Result<FindQuery> {
        let mut query = FindQuery::default();
        query.hidden = true;
        self.apply_to(&mut query)?;
        Ok(query)
    }
}

/// Content selection for the fetch commands (`download`/`verify`): content
/// hashes, a path prefix, and a path glob, plus the shared filter group.
#[derive(Debug, Args, Clone)]
pub struct ContentFilter {
    #[arg(long, help = "Content hash(es) to select (can repeat)")]
    pub hash: Vec<String>,

    #[arg(long, help = "Only entries whose path starts with this prefix")]
    pub path_prefix: Option<String>,

    #[arg(
        long = "path-glob",
        alias = "glob",
        help = "Only entries whose path matches this glob pattern"
    )]
    pub path_glob: Option<String>,

    #[command(flatten)]
    pub content: ContentFilterArgs,
}

impl From<ContentFilterArgs> for ContentFilter {
    fn from(content: ContentFilterArgs) -> Self {
        Self {
            hash: Vec::new(),
            path_prefix: None,
            path_glob: None,
            content,
        }
    }
}

impl ContentFilter {
    /// True when no filter (hash, path, glob, or shared group) is set.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.hash.is_empty() && self.path_prefix.is_none() && self.path_glob.is_none() && self.content.is_empty()
    }
}

impl TryFrom<&ContentFilter> for VerifyFilter {
    type Error = anyhow::Error;

    fn try_from(cf: &ContentFilter) -> std::result::Result<Self, Self::Error> {
        let hashes: Vec<Hash> = cf
            .hash
            .iter()
            .map(|h| {
                h.parse::<Hash>()
                    .map_err(|e| anyhow::anyhow!("invalid content hash {h}: {e}"))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut filter = VerifyFilter::default();
        filter.hashes = if hashes.is_empty() { None } else { Some(hashes) };
        filter.path = cf.path_prefix.clone().map(PathBuf::from);
        filter.glob.clone_from(&cf.path_glob);
        Ok(filter)
    }
}

#[derive(Debug, Args, Clone)]
pub struct ProviderSelector {
    #[arg(long, visible_alias = "provider", help = "Blob ticket(s) for providers (can repeat)")]
    pub from: Vec<String>,

    #[arg(long, default_value_t = 2, help = "Minimum providers for healthy replication")]
    pub min_providers: usize,

    #[arg(long, visible_alias = "no-seeding", help = "Do not share or seed downloaded content")]
    pub no_sharing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_group_is_no_filter() {
        let args = ContentFilterArgs::default();
        assert!(args.is_empty());
        let query = args.to_find_query().expect("empty group builds a query");
        assert!(query.pattern.is_empty());
        assert_eq!(query.min_depth, Some(0));
        assert!(query.max_depth.is_none());
        assert!(query.extensions.is_empty());
        assert!(query.min_size.is_none());
        assert!(query.max_size.is_none());
        assert!(query.file_type.is_none());
        assert!(query.modified_after.is_none());
        assert!(query.modified_before.is_none());
        assert!(query.hidden, "predicate-only queries keep hidden entries");
    }

    #[test]
    fn group_maps_every_find_query_field() {
        let args = ContentFilterArgs {
            ext: vec!["mp4".to_owned(), "mkv".to_owned()],
            size: vec!["+500MB".to_owned()],
            depth: vec!["+1".to_owned()],
            min_depth: None,
            max_depth: Some(3),
            file_type: Some("d".to_owned()),
            modified_within: vec!["7 days".to_owned()],
            ..ContentFilterArgs::default()
        };

        let query = args.to_find_query().expect("group maps to a query");
        assert_eq!(query.extensions, vec!["mp4", "mkv"]);
        assert_eq!(query.min_size, Some(500_000_001));
        assert_eq!(query.max_size, None);
        assert_eq!(query.min_depth, Some(1));
        assert_eq!(query.max_depth, Some(3));
        assert_eq!(query.file_type, Some(FileType::Directory));
        assert!(query.modified_after.is_some());
        assert!(!args.is_empty());
    }

    #[test]
    fn group_converts_into_content_filter() {
        let args = ContentFilterArgs {
            remote_only: true,
            ext: vec!["mp4".to_owned()],
            ..ContentFilterArgs::default()
        };
        let filter = ContentFilter::from(args);
        assert_eq!(filter.content.ext, vec!["mp4".to_owned()]);
        assert!(filter.content.remote_only);
        assert!(filter.hash.is_empty());
        assert!(filter.path_prefix.is_none());
        assert!(filter.path_glob.is_none());
        assert!(!filter.is_empty());
    }

    #[test]
    fn content_filter_hash_only_seed() {
        let filter = ContentFilter {
            hash: vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned()],
            path_prefix: Some("docs".to_owned()),
            path_glob: Some("*.md".to_owned()),
            content: ContentFilterArgs::default(),
        };
        assert!(!filter.is_empty());
        let verify: VerifyFilter = VerifyFilter::try_from(&filter).expect("valid content filter converts");
        assert_eq!(verify.hashes.as_ref().map(Vec::len), Some(1));
        assert_eq!(verify.path, Some(PathBuf::from("docs")));
        assert_eq!(verify.glob, Some("*.md".to_owned()));
    }

    #[test]
    fn old_spellings_parse_as_hidden_aliases() {
        use clap::Parser;

        #[derive(Debug, Parser)]
        struct Probe {
            #[command(flatten)]
            filter: ContentFilterArgs,
        }
        let probe = Probe::try_parse_from([
            "probe",
            "--extension",
            "md",
            "--levels",
            "2",
            "--sizes",
            "+1k",
            "--changed-within",
            "3 days",
        ])
        .expect("old spellings parse as hidden aliases");
        assert_eq!(probe.filter.ext, vec!["md"]);
        assert_eq!(probe.filter.depth, vec!["2"]);
        assert_eq!(probe.filter.size, vec!["+1k"]);
        assert_eq!(probe.filter.modified_within, vec!["3 days"]);
    }
}
