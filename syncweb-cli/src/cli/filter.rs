use std::path::PathBuf;

use clap::Args;
use iroh_blobs::Hash;

use syncweb_core::verify::VerifyFilter;

#[derive(Debug, Args, Clone)]
pub struct ContentFilter {
    #[arg(long, help = "Content hash(es) to select (can repeat)")]
    pub hash: Vec<String>,

    #[arg(long, help = "Only entries whose path starts with this prefix")]
    pub path_prefix: Option<String>,

    #[arg(long, help = "Only entries whose path matches this glob pattern")]
    pub glob: Option<String>,
}

impl ContentFilter {
    pub const fn is_empty(&self) -> bool {
        self.hash.is_empty() && self.path_prefix.is_none() && self.glob.is_none()
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
        filter.glob.clone_from(&cf.glob);
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
