//! Reading text files whose encoding is not known in advance.
//!
//! Markdown and HTML files in the wild are mostly UTF-8, but older HTML pages
//! (and files saved by some Windows editors) use legacy code pages such as
//! Windows-1252 or Windows-1258 (Vietnamese). Converters read input through
//! [`read_text`] so they always work on correct Unicode text.

use std::fs;
use std::path::Path;

use encoding_rs::{Encoding, UTF_8, WINDOWS_1252};

use crate::{ConvertError, Result};

/// How many leading bytes are scanned for an HTML `charset` declaration.
/// Browsers use 1024 as well.
const CHARSET_SCAN_LIMIT: usize = 1024;

/// Reads a text file and decodes it to a `String`.
///
/// The encoding is chosen in this order:
/// 1. a byte-order mark (UTF-8 / UTF-16),
/// 2. an HTML `<meta charset=…>` declaration near the start of the file,
/// 3. UTF-8, if the bytes are valid UTF-8,
/// 4. Windows-1252 as the last resort (never fails).
pub fn read_text(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|e| ConvertError::io(path, e))?;
    Ok(decode(&bytes))
}

/// Decodes raw bytes using the rules described in [`read_text`].
pub fn decode(bytes: &[u8]) -> String {
    let encoding = Encoding::for_bom(bytes)
        .map(|(encoding, _bom_len)| encoding)
        .or_else(|| declared_charset(bytes))
        .unwrap_or_else(|| {
            if std::str::from_utf8(bytes).is_ok() {
                UTF_8
            } else {
                WINDOWS_1252
            }
        });

    // `decode` strips a matching BOM and replaces invalid sequences with U+FFFD.
    let (text, used, had_errors) = encoding.decode(bytes);
    if had_errors {
        tracing::warn!(
            encoding = used.name(),
            "text contained invalid byte sequences"
        );
    }
    text.into_owned()
}

/// Looks for `charset=<label>` (as in `<meta charset="…">` or the
/// `http-equiv` form) in the first [`CHARSET_SCAN_LIMIT`] bytes.
fn declared_charset(bytes: &[u8]) -> Option<&'static Encoding> {
    let head = &bytes[..bytes.len().min(CHARSET_SCAN_LIMIT)];
    let head = head.to_ascii_lowercase();
    let start = find(&head, b"charset")? + b"charset".len();

    let label: Vec<u8> = head[start..]
        .iter()
        .copied()
        .skip_while(|b| b.is_ascii_whitespace() || matches!(b, b'=' | b'"' | b'\''))
        .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':' | b'.'))
        .collect();

    Encoding::for_label(&label)
}

/// Position of the first occurrence of `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_is_the_default() {
        assert_eq!(decode("Xin chào".as_bytes()), "Xin chào");
    }

    #[test]
    fn utf8_bom_is_stripped() {
        assert_eq!(decode(b"\xEF\xBB\xBFhello"), "hello");
    }

    #[test]
    fn utf16_bom_is_honoured() {
        // "hi" in UTF-16LE with BOM.
        assert_eq!(decode(b"\xFF\xFEh\x00i\x00"), "hi");
    }

    #[test]
    fn meta_charset_is_honoured() {
        // 0xE9 is "é" in Windows-1252 but invalid on its own in UTF-8.
        let html = b"<html><head><meta charset=\"windows-1252\"></head><body>caf\xE9</body>";
        assert!(decode(html).contains("café"));
    }

    #[test]
    fn http_equiv_charset_is_honoured() {
        let html =
            b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=ISO-8859-1\">\xE9";
        assert!(decode(html).ends_with('é'));
    }

    #[test]
    fn invalid_utf8_without_declaration_falls_back_to_windows_1252() {
        assert_eq!(decode(b"caf\xE9"), "café");
    }
}
