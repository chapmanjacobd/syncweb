//! Minimal XDR primitives for the bep-relay wire format.
//!
//! XDR (RFC 4506) encodes integers big-endian in four-byte words and
//! zero-pads every value to a four-byte boundary. Byte vectors and strings
//! are a four-byte big-endian length followed by the data and its padding.
//! Upstream reference: `lib/relay/protocol` in the Syncthing repository.

use std::result::Result;

/// Bytes required to make `bytes_len` align to an XDR word boundary.
const fn xdr_padding(bytes_len: usize) -> usize {
    // 4 - (len mod 4), expressed without division or modulo arithmetic.
    match bytes_len & 0b11 {
        0 => 0,
        1 => 3,
        2 => 2,
        3 => 1,
        _ => unreachable!(),
    }
}

/// Append a big-endian `u32` (an XDR word).
pub(super) fn push_u32(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_be_bytes());
}

/// Append a length-prefixed, word-padded byte field.
///
/// # Errors
///
/// Returns an error if `data` is too long to encode.
pub(super) fn push_bytes(buf: &mut Vec<u8>, data: &[u8]) -> Result<(), String> {
    let length = u32::try_from(data.len()).map_err(|error| format!("byte field length exceeds u32::MAX: {error}"))?;
    push_u32(buf, length);
    buf.extend_from_slice(data);
    buf.extend(std::iter::repeat_n(0_u8, xdr_padding(data.len())));
    Ok(())
}

/// Append a big-endian `u16` as a four-byte XDR word (two zero pad bytes,
/// then the value high byte first).
pub(super) fn push_u16(buf: &mut Vec<u8>, value: u16) {
    push_u32(buf, u32::from(value));
}

/// Append a boolean as an XDR word.
pub(super) fn push_bool(buf: &mut Vec<u8>, value: bool) {
    push_u32(buf, u32::from(value));
}

/// A cursor over a message payload, decoding XDR words.
pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Create a reader over `bytes`.
    #[must_use]
    pub(super) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Read the next big-endian `u32`.
    pub(super) fn take_u32(&mut self) -> Result<u32, &'static str> {
        let end = self.pos.checked_add(4).ok_or("offset overflow")?;
        let bytes = self.bytes.get(self.pos..end).ok_or("unexpected end of message")?;
        let mut word = [0_u8; 4];
        word.copy_from_slice(bytes);
        self.pos = end;
        Ok(u32::from_be_bytes(word))
    }

    /// Read the next length-prefixed byte field, rejecting fields longer than
    /// `max`.
    pub(super) fn take_bytes(&mut self, max: usize) -> Result<Vec<u8>, String> {
        let length =
            usize::try_from(self.take_u32()?).map_err(|error| format!("byte field length overflows usize: {error}"))?;
        if length > max {
            return Err(format!("byte field of length {length} exceeds limit {max}"));
        }
        let end = self
            .pos
            .checked_add(length)
            .ok_or_else(|| "byte field length overflows message".to_owned())?;
        let data = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| "byte field extends past the message end".to_owned())?
            .to_vec();
        let padded_end = end
            .checked_add(xdr_padding(length))
            .ok_or_else(|| "padded length overflows".to_owned())?;
        if padded_end > self.bytes.len() {
            return Err("byte field padding extends past the message end".to_owned());
        }
        self.pos = padded_end;
        Ok(data)
    }

    /// Read a `bool` stored as an XDR word.
    pub(super) fn take_bool(&mut self) -> Result<bool, &'static str> {
        Ok(self.take_u32()? != 0)
    }

    /// Assert that the payload has been fully consumed.
    pub(super) const fn finish(&self) -> Result<(), &'static str> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err("trailing bytes after the message fields")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_follows_xdr() {
        assert_eq!(xdr_padding(0), 0);
        assert_eq!(xdr_padding(1), 3);
        assert_eq!(xdr_padding(4), 0);
        assert_eq!(xdr_padding(5), 3);
    }

    #[test]
    fn push_and_take_bytes_round_trip() {
        let mut buf = Vec::new();
        push_bytes(&mut buf, b"ab").expect("small field");
        let mut reader = Reader::new(&buf);
        assert_eq!(reader.take_bytes(8).expect("read"), b"ab".to_vec());
        assert_eq!(buf.len(), 8);
        assert!(reader.finish().is_ok());
    }

    #[test]
    fn oversize_fields_are_rejected() {
        let mut buf = Vec::new();
        push_bytes(&mut buf, b"abcdef").expect("small field");
        let mut reader = Reader::new(&buf);
        assert!(reader.take_bytes(4).is_err());
    }

    #[test]
    fn truncated_fields_are_rejected() {
        let mut buf = Vec::new();
        push_u32(&mut buf, 16);
        let mut reader = Reader::new(&buf);
        assert!(reader.take_bytes(32).is_err());
    }

    #[test]
    fn booleans_and_u16_are_four_byte_words() {
        let mut buf = Vec::new();
        push_u16(&mut buf, 0x0102);
        push_bool(&mut buf, true);
        assert_eq!(buf, vec![0, 0, 1, 2, 0, 0, 0, 1]);
        let mut reader = Reader::new(&buf);
        assert_eq!(reader.take_u32().expect("u16"), 0x0102);
        assert!(reader.take_bool().expect("bool"));
        assert!(reader.finish().is_ok());
    }
}
