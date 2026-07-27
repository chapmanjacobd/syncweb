use std::collections::HashMap;

pub fn detect_mime(params: &HashMap<String, String>, magic_bytes: &[u8]) -> String {
    params
        .get("mime")
        .filter(|m| !m.is_empty())
        .cloned()
        .unwrap_or_else(|| detect_mime_from_magic(magic_bytes).to_string())
}

fn detect_mime_from_magic(data: &[u8]) -> &'static str {
    if data.len() < 4 {
        return "application/octet-stream";
    }

    if starts_with(data, &[0x1A, 0x45, 0xDF, 0xA3]) {
        return "video/webm";
    }

    if data.len() >= 8 && data.get(4..8) == Some(&[0x66, 0x74, 0x79, 0x70]) {
        return "video/mp4";
    }

    if starts_with(data, &[0x66, 0x4C, 0x61, 0x43]) {
        return "audio/flac";
    }

    if starts_with(data, &[0xFF, 0xFB, 0x90, 0x4C]) {
        return "audio/mpeg";
    }

    if starts_with(data, &[0x49, 0x44, 0x33]) {
        return "audio/mpeg";
    }

    if starts_with(data, &[0x4F, 0x67, 0x67, 0x53]) {
        return "audio/ogg";
    }

    if starts_with(data, &[0x52, 0x49, 0x46, 0x46]) {
        if data.len() >= 12 && data.get(8..12) == Some(&[0x57, 0x41, 0x56, 0x45]) {
            return "audio/wav";
        }
        if data.len() >= 12 && data.get(8..12) == Some(&[0x41, 0x56, 0x49, 0x20]) {
            return "video/avi";
        }
        if data.len() >= 12 && data.get(8..12) == Some(&[0x57, 0x45, 0x42, 0x50]) {
            return "image/webp";
        }
        return "application/octet-stream";
    }

    if starts_with(data, &[0x47, 0x40, 0x00]) {
        return "video/mp2t";
    }

    if starts_with(data, &[0x89, 0x50, 0x4E, 0x47]) {
        return "image/png";
    }

    if data.len() >= 2 && data.get(0..2) == Some(&[0xFF, 0xD8]) {
        return "image/jpeg";
    }

    if starts_with(data, &[0x25, 0x50, 0x44, 0x46]) {
        return "application/pdf";
    }

    "application/octet-stream"
}

fn starts_with(data: &[u8], prefix: &[u8]) -> bool {
    data.len() >= prefix.len() && data.get(..prefix.len()) == Some(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_from_query() {
        let mut params = HashMap::new();
        params.insert("mime".to_string(), "video/mp4".to_string());
        assert_eq!(detect_mime(&params, b""), "video/mp4");
    }

    #[test]
    fn test_default_when_no_query() {
        let params = HashMap::new();
        assert_eq!(detect_mime(&params, b""), "application/octet-stream");
    }

    #[test]
    fn test_webm_detection() {
        let data = [0x1A, 0x45, 0xDF, 0xA3, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_mime(&HashMap::new(), &data), "video/webm");
    }

    #[test]
    fn test_mp4_detection() {
        let mut data = vec![0x00, 0x00, 0x00, 0x00, 0x66, 0x74, 0x79, 0x70];
        data.extend_from_slice(b"mp42");
        assert_eq!(detect_mime(&HashMap::new(), &data), "video/mp4");
    }

    #[test]
    fn test_png_detection() {
        let data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_mime(&HashMap::new(), &data), "image/png");
    }

    #[test]
    fn test_jpeg_detection() {
        let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(detect_mime(&HashMap::new(), &data), "image/jpeg");
    }

    #[test]
    fn test_query_overrides_magic() {
        let mut params = HashMap::new();
        params.insert("mime".to_string(), "audio/mpeg".to_string());
        let data = [0x89, 0x50, 0x4E, 0x47];
        assert_eq!(detect_mime(&params, &data), "audio/mpeg");
    }
}
