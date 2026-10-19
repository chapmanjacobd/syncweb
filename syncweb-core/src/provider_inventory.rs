//! Compact, bounded provider inventory encoding.
//!
//! Inventories carry manifest roots, complete blob identifiers, and byte
//! ranges for partially available blobs. The wire format uses a shared,
//! prefix-compressed hash table so large collections do not repeat the same
//! 32-byte identifiers in every section.

use std::cmp::Ordering;

use iroh_blobs::Hash;

use crate::{Result, SyncwebError};

pub const PROVIDER_INVENTORY_VERSION: u8 = 1;

const MAGIC: &[u8; 4] = b"SWIV";
const HASH_SIZE: usize = 32;
const MAX_HASHES: usize = 1_000_000;
const MAX_RANGES: usize = 1_000_000;
const MAX_ENCODED_SIZE: usize = 16 * 1024 * 1024;

/// A byte range available for a content-addressed blob.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct InventoryRange {
    pub hash: Hash,
    pub offset: u64,
    pub length: u64,
}

impl InventoryRange {
    /// Construct a non-empty, non-overflowing blob range.
    ///
    /// # Errors
    ///
    /// Returns an error when `length` is zero or the range exceeds `u64::MAX`.
    pub fn new(hash: Hash, offset: u64, length: u64) -> Result<Self> {
        if length == 0 {
            return Err(SyncwebError::InvalidConfig(
                "provider inventory ranges must be non-empty".to_owned(),
            ));
        }
        offset
            .checked_add(length)
            .ok_or_else(|| SyncwebError::InvalidConfig("provider inventory range overflows u64".to_owned()))?;
        Ok(Self { hash, offset, length })
    }
}

/// Content availability summarized for provider discovery and reconciliation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProviderInventory {
    /// Manifest or collection roots known by the provider.
    pub roots: Vec<Hash>,
    /// Complete blobs known by the provider.
    pub blobs: Vec<Hash>,
    /// Independently available ranges of blobs.
    pub ranges: Vec<InventoryRange>,
}

impl ProviderInventory {
    #[must_use]
    pub const fn new(roots: Vec<Hash>, blobs: Vec<Hash>, ranges: Vec<InventoryRange>) -> Self {
        Self { roots, blobs, ranges }
    }

    /// Encode the inventory into its bounded binary wire representation.
    ///
    /// Hashes are deduplicated and sorted before encoding, making equivalent
    /// inventories produce identical bytes and allowing ranges to reference a
    /// shared hash table.
    ///
    /// # Errors
    ///
    /// Returns an error when the inventory exceeds configured bounds or
    /// contains an invalid range.
    pub fn encode(&self) -> Result<Vec<u8>> {
        validate_ranges(&self.ranges)?;
        let hashes = self.sorted_hashes();
        if hashes.len() > MAX_HASHES
            || self.roots.len() > MAX_HASHES
            || self.blobs.len() > MAX_HASHES
            || self.ranges.len() > MAX_RANGES
        {
            return Err(SyncwebError::InvalidConfig(
                "provider inventory exceeds the maximum number of entries".to_owned(),
            ));
        }

        let mut output = Vec::new();
        output.extend_from_slice(MAGIC);
        output.push(PROVIDER_INVENTORY_VERSION);
        write_varint(
            u64::try_from(hashes.len())
                .map_err(|error| SyncwebError::operation("provider inventory hash count overflowed", error))?,
            &mut output,
        );
        write_varint(
            u64::try_from(self.roots.len())
                .map_err(|error| SyncwebError::operation("provider inventory root count overflowed", error))?,
            &mut output,
        );
        write_varint(
            u64::try_from(self.blobs.len())
                .map_err(|error| SyncwebError::operation("provider inventory blob count overflowed", error))?,
            &mut output,
        );
        write_varint(
            u64::try_from(self.ranges.len())
                .map_err(|error| SyncwebError::operation("provider inventory range count overflowed", error))?,
            &mut output,
        );

        let mut previous = [0_u8; HASH_SIZE];
        for hash in &hashes {
            let bytes = hash.as_bytes();
            let common = common_prefix(&previous, bytes);
            output.push(
                u8::try_from(common)
                    .map_err(|error| SyncwebError::operation("provider inventory prefix overflowed", error))?,
            );
            let suffix = bytes
                .get(common..)
                .ok_or_else(|| SyncwebError::InvalidConfig("provider inventory prefix is invalid".to_owned()))?;
            output.extend_from_slice(suffix);
            previous.copy_from_slice(bytes);
        }

        for hash in &self.roots {
            write_index(hash, &hashes, &mut output)?;
        }
        for hash in &self.blobs {
            write_index(hash, &hashes, &mut output)?;
        }
        for range in &self.ranges {
            write_index(&range.hash, &hashes, &mut output)?;
            write_varint(range.offset, &mut output);
            write_varint(range.length, &mut output);
        }

        if output.len() > MAX_ENCODED_SIZE {
            return Err(SyncwebError::InvalidConfig(
                "provider inventory exceeds the maximum encoded size".to_owned(),
            ));
        }
        Ok(output)
    }

    /// Decode and validate a provider inventory.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported version, malformed varints,
    /// truncated data, invalid references, non-canonical hash ordering, or
    /// overflowing ranges.
    pub fn decode(encoded: &[u8]) -> Result<Self> {
        if encoded.len() > MAX_ENCODED_SIZE {
            return Err(SyncwebError::InvalidConfig(
                "provider inventory exceeds the maximum encoded size".to_owned(),
            ));
        }
        let mut reader = Reader::new(encoded);
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(SyncwebError::InvalidConfig(
                "provider inventory has an invalid magic header".to_owned(),
            ));
        }
        if reader.byte()? != PROVIDER_INVENTORY_VERSION {
            return Err(SyncwebError::InvalidConfig(
                "unsupported provider inventory version".to_owned(),
            ));
        }
        let hash_count = bounded_count(reader.varint()?, MAX_HASHES, "hash")?;
        let root_count = bounded_count(reader.varint()?, MAX_HASHES, "root")?;
        let blob_count = bounded_count(reader.varint()?, MAX_HASHES, "blob")?;
        let range_count = bounded_count(reader.varint()?, MAX_RANGES, "range")?;

        let mut hashes = Vec::with_capacity(hash_count);
        let mut previous = [0_u8; HASH_SIZE];
        for _ in 0..hash_count {
            let common = usize::from(reader.byte()?);
            if common > HASH_SIZE {
                return Err(SyncwebError::InvalidConfig(
                    "provider inventory hash prefix is too long".to_owned(),
                ));
            }
            let suffix = reader.take(HASH_SIZE.saturating_sub(common))?;
            let mut bytes = previous;
            bytes
                .get_mut(common..)
                .ok_or_else(|| SyncwebError::InvalidConfig("provider inventory prefix is invalid".to_owned()))?
                .copy_from_slice(suffix);
            let hash = Hash::from_bytes(bytes);
            if hashes
                .last()
                .is_some_and(|last: &Hash| compare_hash(last, &hash) != Ordering::Less)
            {
                return Err(SyncwebError::InvalidConfig(
                    "provider inventory hash table is not strictly sorted".to_owned(),
                ));
            }
            hashes.push(hash);
            previous = bytes;
        }

        let roots = read_hash_indices(&mut reader, &hashes, root_count)?;
        let blobs = read_hash_indices(&mut reader, &hashes, blob_count)?;
        let mut ranges = Vec::with_capacity(range_count);
        for _ in 0..range_count {
            let hash = read_hash_index(&mut reader, &hashes)?;
            let offset = reader.varint()?;
            let length = reader.varint()?;
            ranges.push(InventoryRange::new(hash, offset, length)?);
        }
        reader.finish()?;
        Ok(Self { roots, blobs, ranges })
    }

    fn sorted_hashes(&self) -> Vec<Hash> {
        let mut hashes = self
            .roots
            .iter()
            .copied()
            .chain(self.blobs.iter().copied())
            .chain(self.ranges.iter().map(|range| range.hash))
            .collect::<Vec<_>>();
        hashes.sort_by(compare_hash);
        hashes.dedup();
        hashes
    }
}

fn validate_ranges(ranges: &[InventoryRange]) -> Result<()> {
    if ranges.len() > MAX_RANGES {
        return Err(SyncwebError::InvalidConfig(
            "provider inventory exceeds the maximum number of ranges".to_owned(),
        ));
    }
    for range in ranges {
        if range.length == 0 || range.offset.checked_add(range.length).is_none() {
            return Err(SyncwebError::InvalidConfig(
                "provider inventory contains an invalid range".to_owned(),
            ));
        }
    }
    Ok(())
}

fn compare_hash(left: &Hash, right: &Hash) -> Ordering {
    left.as_bytes().cmp(right.as_bytes())
}

fn common_prefix(left: &[u8; HASH_SIZE], right: &[u8; HASH_SIZE]) -> usize {
    left.iter()
        .zip(right.iter())
        .position(|(left_byte, right_byte)| left_byte != right_byte)
        .unwrap_or(HASH_SIZE)
}

fn write_index(hash: &Hash, hashes: &[Hash], output: &mut Vec<u8>) -> Result<()> {
    let index = hashes
        .binary_search_by(|candidate| compare_hash(candidate, hash))
        .map_err(|error| {
            SyncwebError::InvalidConfig(format!("provider inventory hash reference is missing: {error}"))
        })?;
    write_varint(
        u64::try_from(index).map_err(|error| SyncwebError::operation("provider inventory index overflowed", error))?,
        output,
    );
    Ok(())
}

fn read_hash_indices(reader: &mut Reader<'_>, hashes: &[Hash], count: usize) -> Result<Vec<Hash>> {
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        result.push(read_hash_index(reader, hashes)?);
    }
    Ok(result)
}

fn read_hash_index(reader: &mut Reader<'_>, hashes: &[Hash]) -> Result<Hash> {
    let index = usize::try_from(reader.varint()?)
        .map_err(|error| SyncwebError::InvalidConfig(format!("provider inventory index is invalid: {error}")))?;
    hashes
        .get(index)
        .copied()
        .ok_or_else(|| SyncwebError::InvalidConfig("provider inventory references an unknown hash".to_owned()))
}

fn bounded_count(value: u64, maximum: usize, kind: &str) -> Result<usize> {
    let count = usize::try_from(value)
        .map_err(|error| SyncwebError::InvalidConfig(format!("provider inventory {kind} count is invalid: {error}")))?;
    if count > maximum {
        return Err(SyncwebError::InvalidConfig(format!(
            "provider inventory {kind} count exceeds {maximum}"
        )));
    }
    Ok(count)
}

fn write_varint(mut value: u64, output: &mut Vec<u8>) {
    while value >= 0x80 {
        output.push(value.to_le_bytes()[0] | 0x80);
        value >>= 7;
    }
    output.push(value.to_le_bytes()[0]);
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn byte(&mut self) -> Result<u8> {
        let byte = self
            .bytes
            .get(self.offset)
            .copied()
            .ok_or_else(|| SyncwebError::InvalidConfig("provider inventory is truncated".to_owned()))?;
        self.offset = self.offset.saturating_add(1);
        Ok(byte)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| SyncwebError::InvalidConfig("provider inventory offset overflowed".to_owned()))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| SyncwebError::InvalidConfig("provider inventory is truncated".to_owned()))?;
        self.offset = end;
        Ok(bytes)
    }

    fn varint(&mut self) -> Result<u64> {
        let mut value = 0_u64;
        for shift in (0..70).step_by(7) {
            let byte = self.byte()?;
            let payload = u64::from(byte & 0x7f);
            value =
                value
                    .checked_add(payload.checked_shl(shift).ok_or_else(|| {
                        SyncwebError::InvalidConfig("provider inventory varint overflows u64".to_owned())
                    })?)
                    .ok_or_else(|| SyncwebError::InvalidConfig("provider inventory varint overflows u64".to_owned()))?;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(SyncwebError::InvalidConfig(
            "provider inventory varint is too long".to_owned(),
        ))
    }

    fn finish(self) -> Result<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(SyncwebError::InvalidConfig(
                "provider inventory has trailing bytes".to_owned(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn hash(byte: u8) -> Hash {
        Hash::from_bytes([byte; HASH_SIZE])
    }

    #[test]
    fn round_trip_is_canonical_and_deduplicated() {
        let range = InventoryRange::new(hash(2), 10, 20).expect("valid range");
        let inventory = ProviderInventory::new(vec![hash(3), hash(1), hash(3)], vec![hash(2), hash(1)], vec![range]);
        let encoded = inventory.encode().expect("inventory should encode");
        let decoded = ProviderInventory::decode(&encoded).expect("inventory should decode");
        assert_eq!(decoded, inventory);
        assert_eq!(encoded, inventory.encode().expect("inventory should re-encode"));
        assert!(encoded.len() < 4 + 1 + 4 + 5 * HASH_SIZE);
    }

    #[test]
    fn rejects_truncation_and_trailing_bytes() {
        let encoded = ProviderInventory::new(vec![hash(1)], Vec::new(), Vec::new())
            .encode()
            .expect("inventory should encode");
        let mut truncated = encoded.clone();
        truncated.pop();
        assert!(ProviderInventory::decode(&truncated).is_err());
        let mut trailing = encoded;
        trailing.push(0);
        assert!(ProviderInventory::decode(&trailing).is_err());
    }

    #[test]
    fn rejects_invalid_ranges() {
        assert!(InventoryRange::new(hash(1), 0, 0).is_err());
        assert!(InventoryRange::new(hash(1), u64::MAX, 2).is_err());
    }
}
