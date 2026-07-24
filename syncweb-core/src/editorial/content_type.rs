use serde::{Deserialize, Serialize};

/// Structured content type taxonomy.
///
/// When absent the collection behaves as a generic file set (default).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[derive(Default)]
pub enum ContentType {
    /// Default behaviour — no schema validation, freeform entries.
    #[default]
    Generic,
    /// Article-style content (title, body, author, tags).
    Article(ArticleMetadata),
    /// Structured dataset (source, format, columns, license).
    Dataset(DatasetMetadata),
    /// Audio/video media (codec, duration, resolution, subtitles).
    Media(MediaMetadata),
    /// Documentation (markdown/reST with TOC, version hints).
    Documentation(DocumentationMetadata),
    /// User-defined type with an optional JSON schema reference.
    Custom {
        /// Human-readable type label.
        name: String,
        /// Optional JSON schema blob hash for validation.
        schema: Option<String>,
    },
}

impl ContentType {
    /// A short label suitable for CLI output.
    #[must_use]
    pub const fn label(&self) -> &str {
        match self {
            Self::Generic => "generic",
            Self::Article(_) => "article",
            Self::Dataset(_) => "dataset",
            Self::Media(_) => "media",
            Self::Documentation(_) => "documentation",
            Self::Custom { name, .. } => name.as_str(),
        }
    }

    /// Whether this type carries inline metadata (article, dataset, etc.).
    #[must_use]
    pub const fn is_generic(&self) -> bool {
        matches!(self, Self::Generic)
    }
}

/// Metadata describing an article-style collection.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ArticleMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub language: Option<String>,
    pub tags: Vec<String>,
}

/// Metadata describing a structured dataset.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DatasetMetadata {
    pub source: Option<String>,
    pub format: Option<String>,
    pub license: Option<String>,
    pub columns: Vec<String>,
    pub record_count: Option<u64>,
}

/// Metadata describing audio/video media.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MediaMetadata {
    pub media_kind: Option<String>,
    pub codec: Option<String>,
    pub duration_seconds: Option<u64>,
    pub resolution: Option<String>,
    pub subtitles: Vec<String>,
}

/// Metadata describing documentation content.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DocumentationMetadata {
    pub format: Option<String>,
    pub version: Option<String>,
    pub sections: Vec<String>,
}
