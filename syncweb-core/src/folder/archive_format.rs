use std::collections::BTreeSet;

use iroh_blobs::Hash;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Result, SyncwebError, net::CommunityId};

use super::{CollectionEntry, CollectionManifest};

pub(crate) const CAR_VERSION: u64 = 1;
pub(crate) const CID_VERSION: u64 = 1;
pub(crate) const RAW_CODEC: u64 = 0x55;
pub(crate) const BLAKE3_MULTIHASH: u64 = 0x1e;
pub(crate) const HASH_SIZE: u64 = 32;
pub(crate) const DROP_INDEX_VERSION: u8 = 1;

/// Portable metadata describing the contents and provenance of a drop archive.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DropIndex {
    pub version: u8,
    pub roots: Vec<Hash>,
    pub manifest: Hash,
    pub collection_id: Uuid,
    pub collection_version: String,
    pub entry_count: usize,
    pub blocks: Vec<DropIndexBlock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_community: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<DropProviderHint>,
    #[serde(default)]
    pub mode: String,
}

/// One content block listed by a drop index.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DropIndexBlock {
    pub hash: Hash,
    pub size: u64,
    pub paths: Vec<String>,
}

/// Optional provider capability information carried by a drop index.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DropProviderHint {
    pub provider: String,
    pub supports_segments: bool,
    pub supports_relay: bool,
}

impl DropProviderHint {
    /// Construct a provider capability hint for an archive index.
    #[must_use]
    pub fn new(provider: impl Into<String>, supports_segments: bool, supports_relay: bool) -> Self {
        Self {
            provider: provider.into(),
            supports_segments,
            supports_relay,
        }
    }
}

impl DropIndex {
    pub(crate) fn from_manifest(
        manifest: &CollectionManifest,
        manifest_hash: Hash,
        entries: &[CollectionEntry],
        mode: impl Into<String>,
        source_community: Option<String>,
        providers: Vec<DropProviderHint>,
    ) -> Result<Self> {
        validate_source_community(manifest, source_community.as_deref())?;
        let mut roots = vec![manifest_hash];
        if let Some(parent) = manifest.parent
            && parent != manifest_hash
        {
            roots.push(parent);
        }
        let mut blocks = Vec::new();
        let mut seen = BTreeSet::new();
        for entry in entries {
            if !seen.insert(entry.content_id) {
                continue;
            }
            blocks.push(DropIndexBlock {
                hash: entry.content_id,
                size: entry.size,
                paths: manifest
                    .entries
                    .iter()
                    .filter(|candidate| candidate.content_id == entry.content_id)
                    .map(|candidate| candidate.logical_path.to_string_lossy().replace('\\', "/"))
                    .collect(),
            });
        }
        Ok(Self {
            version: DROP_INDEX_VERSION,
            roots,
            manifest: manifest_hash,
            collection_id: manifest.collection_id,
            collection_version: manifest.version.clone(),
            entry_count: manifest.entries.len(),
            blocks,
            source_community,
            providers,
            mode: mode.into(),
        })
    }

    /// Serialize the index for storage as a CAR block.
    ///
    /// # Errors
    ///
    /// Returns an error when the index version is unsupported or serialization
    /// fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        if self.version != DROP_INDEX_VERSION {
            return Err(SyncwebError::InvalidConfig(format!(
                "unsupported drop index version {}",
                self.version
            )));
        }
        serde_json::to_vec(self).map_err(|error| SyncwebError::operation("failed to serialize drop index", error))
    }

    /// Decode and validate an index block.
    ///
    /// # Errors
    ///
    /// Returns an error when the index is malformed or uses an unsupported
    /// version.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let index: Self = serde_json::from_slice(bytes)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid drop index: {error}")))?;
        if index.version != DROP_INDEX_VERSION {
            return Err(SyncwebError::InvalidConfig(format!(
                "unsupported drop index version {}",
                index.version
            )));
        }
        if index.roots.first().copied() != Some(index.manifest) {
            return Err(SyncwebError::InvalidConfig(
                "drop index manifest is not its first root".to_owned(),
            ));
        }
        Ok(index)
    }

    /// Validate the optional archive scope against its signed manifest.
    ///
    /// An index may omit `source_community` for compatibility with older
    /// archives, but it must never claim a different community than the
    /// manifest it accompanies.
    ///
    /// # Errors
    ///
    /// Returns an error when the index scope is malformed or disagrees with
    /// the manifest scope.
    pub fn validate_for_manifest(&self, manifest: &CollectionManifest) -> Result<()> {
        validate_source_community(manifest, self.source_community.as_deref())
    }
}

fn validate_source_community(manifest: &CollectionManifest, source_community: Option<&str>) -> Result<()> {
    let Some(source_community) = source_community else {
        return Ok(());
    };
    let manifest_community = manifest.community()?;
    let source_id = match source_community.parse::<CommunityId>() {
        Ok(source_id) => source_id,
        Err(_) if manifest_community.is_none() => {
            // Legacy archives used a human-readable source label. Keep
            // accepting those labels for unscoped manifests.
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    if manifest_community != Some(source_id) {
        return Err(SyncwebError::AccessDenied(
            "drop index community does not match the collection manifest".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn car_header(roots: &[Hash]) -> Result<Vec<u8>> {
    if roots.is_empty() {
        return Err(SyncwebError::InvalidConfig(
            "CAR header requires at least one root".to_owned(),
        ));
    }
    let mut header = Vec::new();
    for root in roots {
        let cid = cid_for_hash(*root);
        let cid_length =
            u64::try_from(cid.len()).map_err(|error| SyncwebError::operation("CAR root CID is too large", error))?;
        header.push(0x0a);
        encode_varint(cid_length, &mut header);
        header.extend_from_slice(&cid);
    }
    header.push(0x10);
    encode_varint(CAR_VERSION, &mut header);
    let header_length =
        u64::try_from(header.len()).map_err(|error| SyncwebError::operation("CAR header is too large", error))?;
    let mut encoded = Vec::with_capacity(header.len().saturating_add(10));
    encode_varint(header_length, &mut encoded);
    encoded.extend_from_slice(&header);
    Ok(encoded)
}

pub(crate) fn parse_car_header(header: &[u8]) -> Result<Vec<Hash>> {
    let mut offset = 0_usize;
    let mut roots = Vec::new();
    loop {
        let field = *header
            .get(offset)
            .ok_or_else(|| SyncwebError::InvalidConfig("CAR header is missing its version".to_owned()))?;
        if field == 0x10 {
            offset = offset
                .checked_add(1)
                .ok_or_else(|| SyncwebError::operation("CAR header offset overflow", "usize limit exceeded"))?;
            break;
        }
        if field != 0x0a {
            return Err(SyncwebError::InvalidConfig(
                "CAR header contains an unsupported field".to_owned(),
            ));
        }
        offset = offset
            .checked_add(1)
            .ok_or_else(|| SyncwebError::operation("CAR header offset overflow", "usize limit exceeded"))?;
        let cid_length = usize::try_from(read_slice_varint(header, &mut offset, "CAR root CID length")?)
            .map_err(|error| SyncwebError::operation("CAR root CID length is not representable", error))?;
        let cid_end = offset
            .checked_add(cid_length)
            .ok_or_else(|| SyncwebError::operation("CAR root CID range overflow", "usize limit exceeded"))?;
        let cid = header
            .get(offset..cid_end)
            .ok_or_else(|| SyncwebError::InvalidConfig("CAR root CID is truncated".to_owned()))?;
        offset = cid_end;
        roots.push(parse_cid(cid)?);
    }
    if roots.is_empty()
        || read_slice_varint(header, &mut offset, "CAR version")? != CAR_VERSION
        || offset != header.len()
    {
        return Err(SyncwebError::InvalidConfig(
            "unsupported or malformed CAR header".to_owned(),
        ));
    }
    Ok(roots)
}

pub(crate) fn cid_for_hash(hash: Hash) -> Vec<u8> {
    let mut cid = Vec::with_capacity(2 + 2 + 2 + 32);
    encode_varint(CID_VERSION, &mut cid);
    encode_varint(RAW_CODEC, &mut cid);
    encode_varint(BLAKE3_MULTIHASH, &mut cid);
    encode_varint(HASH_SIZE, &mut cid);
    cid.extend_from_slice(hash.as_bytes());
    cid
}

pub(crate) fn encode_varint(mut value: u64, output: &mut Vec<u8>) {
    while value >= 0x80 {
        output.push(value.to_le_bytes()[0] | 0x80);
        value >>= 7;
    }
    output.push(value.to_le_bytes()[0]);
}

fn parse_cid(cid: &[u8]) -> Result<Hash> {
    let mut offset = 0_usize;
    let version = read_slice_varint(cid, &mut offset, "CID version")?;
    let codec = read_slice_varint(cid, &mut offset, "CID codec")?;
    let hash_code = read_slice_varint(cid, &mut offset, "CID multihash code")?;
    let hash_size = read_slice_varint(cid, &mut offset, "CID multihash length")?;
    if version != CID_VERSION || codec != RAW_CODEC || hash_code != BLAKE3_MULTIHASH || hash_size != HASH_SIZE {
        return Err(SyncwebError::InvalidConfig(
            "drop contains an unsupported root content identifier".to_owned(),
        ));
    }
    let hash_end = offset
        .checked_add(
            usize::try_from(HASH_SIZE)
                .map_err(|error| SyncwebError::operation("CID hash length is not representable", error))?,
        )
        .ok_or_else(|| SyncwebError::operation("CID hash range overflow", "usize limit exceeded"))?;
    let digest = cid
        .get(offset..hash_end)
        .ok_or_else(|| SyncwebError::InvalidConfig("CID digest is truncated".to_owned()))?;
    if hash_end != cid.len() {
        return Err(SyncwebError::InvalidConfig("CID contains trailing bytes".to_owned()));
    }
    let hash_bytes: [u8; 32] = digest
        .try_into()
        .map_err(|error| SyncwebError::InvalidConfig(format!("CID digest must be 32 bytes: {error}")))?;
    Ok(Hash::from_bytes(hash_bytes))
}

fn read_slice_varint(bytes: &[u8], offset: &mut usize, name: &'static str) -> Result<u64> {
    let mut value = 0_u64;
    let mut shift = 0_u32;
    for index in 0..10 {
        let byte = *bytes
            .get(*offset)
            .ok_or_else(|| SyncwebError::InvalidConfig(format!("{name} is truncated")))?;
        *offset = offset
            .checked_add(1)
            .ok_or_else(|| SyncwebError::operation("CAR offset overflow", name))?;
        let bits = u64::from(byte & 0x7f);
        if shift == 63 && bits > 1 {
            return Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")));
        }
        value |= bits << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        if index == 9 {
            return Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")));
        }
        shift = shift
            .checked_add(7)
            .ok_or_else(|| SyncwebError::operation("CAR varint shift overflow", name))?;
    }
    Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")))
}
