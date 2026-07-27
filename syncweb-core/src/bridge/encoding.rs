use crate::error::{Result, SyncwebError};

use super::service::ConnectedPeer;

/// # Errors
///
/// Returns an error if the buffer is too short.
pub fn read_u16(buf: &[u8], offset: &mut usize) -> Result<u16> {
    let end = offset.wrapping_add(2);
    let arr: [u8; 2] = buf
        .get(*offset..end)
        .ok_or_else(|| SyncwebError::operation("frame parse error", "truncated u16"))?
        .try_into()
        .map_err(|error: std::array::TryFromSliceError| SyncwebError::operation("frame parse error", error))?;
    *offset = end;
    Ok(u16::from_be_bytes(arr))
}

/// # Errors
///
/// Returns an error if the buffer is too short.
pub fn read_u32(buf: &[u8], offset: &mut usize) -> Result<u32> {
    let end = offset.wrapping_add(4);
    let arr: [u8; 4] = buf
        .get(*offset..end)
        .ok_or_else(|| SyncwebError::operation("frame parse error", "truncated u32"))?
        .try_into()
        .map_err(|error: std::array::TryFromSliceError| SyncwebError::operation("frame parse error", error))?;
    *offset = end;
    Ok(u32::from_be_bytes(arr))
}

/// # Errors
///
/// Returns an error if the buffer is too short.
pub fn read_u64(buf: &[u8], offset: &mut usize) -> Result<u64> {
    let end = offset.wrapping_add(8);
    let arr: [u8; 8] = buf
        .get(*offset..end)
        .ok_or_else(|| SyncwebError::operation("frame parse error", "truncated u64"))?
        .try_into()
        .map_err(|error: std::array::TryFromSliceError| SyncwebError::operation("frame parse error", error))?;
    *offset = end;
    Ok(u64::from_be_bytes(arr))
}

/// # Errors
///
/// Returns an error if the buffer is too short or the string is not valid
/// UTF-8.
pub fn read_string(buf: &[u8], offset: &mut usize) -> Result<String> {
    let len = read_u16(buf, offset)?;
    let end = offset.wrapping_add(usize::from(len));
    let slice = buf
        .get(*offset..end)
        .ok_or_else(|| SyncwebError::operation("frame parse error", "truncated string"))?;
    *offset = end;
    let s = String::from_utf8(slice.to_vec()).map_err(|error| SyncwebError::operation("frame parse error", error))?;
    Ok(s)
}

/// # Errors
///
/// Returns an error if the buffer is too short.
pub fn read_bytes(buf: &[u8], offset: &mut usize) -> Result<Vec<u8>> {
    let len = read_u32(buf, offset)?;
    let len_usize = usize::try_from(len).unwrap_or(usize::MAX);
    let end = offset.wrapping_add(len_usize);
    let slice = buf
        .get(*offset..end)
        .ok_or_else(|| SyncwebError::operation("frame parse error", "truncated bytes"))?;
    *offset = end;
    Ok(slice.to_vec())
}

pub fn write_u16(buf: &mut Vec<u8>, val: u16) {
    buf.extend_from_slice(&val.to_be_bytes());
}

pub fn write_u32(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_be_bytes());
}

pub fn write_u64(buf: &mut Vec<u8>, val: u64) {
    buf.extend_from_slice(&val.to_be_bytes());
}

pub fn write_string(buf: &mut Vec<u8>, val: &str) {
    let len = u16::try_from(val.len()).unwrap_or(0);
    write_u16(buf, len);
    buf.extend_from_slice(val.as_bytes());
}

pub fn write_bytes(buf: &mut Vec<u8>, val: &[u8]) {
    let len = u32::try_from(val.len()).unwrap_or(0);
    write_u32(buf, len);
    buf.extend_from_slice(val);
}

pub fn write_peer_list(buf: &mut Vec<u8>, peers: &[ConnectedPeer]) {
    let count = u16::try_from(peers.len()).unwrap_or(0);
    write_u16(buf, count);
    for peer in peers {
        write_string(buf, &peer.node_id);
        write_u64(buf, peer.first_seen_secs);
        write_u64(buf, peer.last_seen_secs);
    }
}

pub fn write_string_list(buf: &mut Vec<u8>, list: &[String]) {
    let count = u16::try_from(list.len()).unwrap_or(0);
    write_u16(buf, count);
    for entry in list {
        write_string(buf, entry);
    }
}
