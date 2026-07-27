use axum::http::HeaderMap;

#[derive(Debug, Clone, Copy)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}

pub fn parse_range_header(headers: &HeaderMap, total_size: u64) -> Option<ByteRange> {
    let header_value = headers.get(axum::http::header::RANGE)?.to_str().ok()?;

    let remainder = header_value.strip_prefix("bytes=")?;

    let (start_str, end_str) = remainder.split_once('-')?;

    let start: u64 = start_str.parse().ok()?;

    let end: u64 = if end_str.is_empty() {
        total_size.saturating_sub(1)
    } else {
        end_str.parse().ok()?
    };

    if start > end || start >= total_size {
        return None;
    }

    Some(ByteRange {
        start,
        end: end.min(total_size.saturating_sub(1)),
    })
}

pub fn content_range(start: u64, end: u64, total_size: u64) -> String {
    format!("bytes {start}-{end}/{total_size}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with_range(range: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(axum::http::header::RANGE, range.parse().unwrap());
        h
    }

    #[test]
    fn test_simple_range() {
        let h = headers_with_range("bytes=0-1023");
        let r = parse_range_header(&h, 5000);
        assert!(r.is_some());
        let r_val = r.unwrap();
        assert_eq!(r_val.start, 0);
        assert_eq!(r_val.end, 1023);
    }

    #[test]
    fn test_open_ended_range() {
        let h = headers_with_range("bytes=2000-");
        let r = parse_range_header(&h, 5000);
        assert!(r.is_some());
        let r_val = r.unwrap();
        assert_eq!(r_val.start, 2000);
        assert_eq!(r_val.end, 4999);
    }

    #[test]
    fn test_no_range_header() {
        let h = HeaderMap::new();
        assert!(parse_range_header(&h, 1000).is_none());
    }

    #[test]
    fn test_malformed_header() {
        let h = headers_with_range("not-bytes=0-100");
        assert!(parse_range_header(&h, 1000).is_none());
    }

    #[test]
    fn test_content_range_format() {
        assert_eq!(content_range(0, 1023, 5000), "bytes 0-1023/5000");
    }
}
