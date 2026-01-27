//! PDF String objects
//!
//! PDF supports two types of strings:
//! - Literal strings: enclosed in parentheses (Hello World)
//! - Hexadecimal strings: enclosed in angle brackets <48656C6C6F>

use std::fmt;

/// A PDF literal string
///
/// Literal strings are enclosed in parentheses and can contain
/// escape sequences like \n, \r, \t, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfString(Vec<u8>);

impl PdfString {
    /// Creates a new PdfString from bytes
    pub fn new(bytes: Vec<u8>) -> Self {
        PdfString(bytes)
    }

    /// Creates a new PdfString from a string (raw bytes, ASCII only)
    ///
    /// This copies UTF-8 bytes directly. For non-ASCII text in metadata,
    /// use `from_text()` instead which properly encodes as UTF-16BE.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        PdfString(s.as_bytes().to_vec())
    }

    /// Creates a new PdfString from text with proper PDF text string encoding
    ///
    /// For ASCII-only text, uses PDFDocEncoding (same as ASCII for 0x00-0x7F).
    /// For text containing non-ASCII characters, uses UTF-16BE with BOM.
    /// This is the correct method for Info dictionary, bookmarks, form fields, etc.
    pub fn from_text(s: &str) -> Self {
        // Check if all characters are ASCII
        if s.is_ascii() {
            // ASCII text can use PDFDocEncoding directly
            PdfString(s.as_bytes().to_vec())
        } else {
            // Non-ASCII text must use UTF-16BE with BOM
            let mut bytes = vec![0xFE, 0xFF]; // BOM
            for ch in s.encode_utf16() {
                bytes.extend_from_slice(&ch.to_be_bytes());
            }
            PdfString(bytes)
        }
    }

    /// Returns the string data as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Returns the string data as a UTF-8 string if valid
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.0).ok()
    }

    /// Decodes the string to a Rust String
    ///
    /// Attempts to decode as UTF-16BE if it starts with BOM, otherwise UTF-8
    pub fn decode_text(&self) -> String {
        // Check for UTF-16BE BOM
        if self.0.len() >= 2 && self.0[0] == 0xFE && self.0[1] == 0xFF {
            // UTF-16BE encoded
            let utf16: Vec<u16> = self.0[2..]
                .chunks(2)
                .filter_map(|chunk| {
                    if chunk.len() == 2 {
                        Some(u16::from_be_bytes([chunk[0], chunk[1]]))
                    } else {
                        None
                    }
                })
                .collect();
            String::from_utf16_lossy(&utf16)
        } else {
            // Assume PDFDocEncoding or Latin-1 for simplicity
            String::from_utf8_lossy(&self.0).into_owned()
        }
    }

    /// Encodes the string for PDF output
    pub fn encode(&self) -> String {
        let mut result = String::with_capacity(self.0.len() * 2 + 2);
        result.push('(');

        for &byte in &self.0 {
            match byte {
                b'\n' => result.push_str("\\n"),
                b'\r' => result.push_str("\\r"),
                b'\t' => result.push_str("\\t"),
                b'\x08' => result.push_str("\\b"),
                b'\x0C' => result.push_str("\\f"),
                b'(' => result.push_str("\\("),
                b')' => result.push_str("\\)"),
                b'\\' => result.push_str("\\\\"),
                0x20..=0x7E => result.push(byte as char),
                _ => {
                    // Encode as octal
                    result.push_str(&format!("\\{:03o}", byte));
                }
            }
        }

        result.push(')');
        result
    }
}

impl fmt::Display for PdfString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl From<&str> for PdfString {
    fn from(s: &str) -> Self {
        PdfString::from_str(s)
    }
}

impl From<String> for PdfString {
    fn from(s: String) -> Self {
        PdfString::new(s.into_bytes())
    }
}

impl From<Vec<u8>> for PdfString {
    fn from(bytes: Vec<u8>) -> Self {
        PdfString::new(bytes)
    }
}

/// A PDF hexadecimal string
///
/// Hexadecimal strings are enclosed in angle brackets and contain
/// hexadecimal digits representing bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfHexString(Vec<u8>);

impl PdfHexString {
    /// Creates a new PdfHexString from bytes
    pub fn new(bytes: Vec<u8>) -> Self {
        PdfHexString(bytes)
    }

    /// Creates a new PdfHexString from a text string (UTF-16BE encoded)
    pub fn from_text(text: &str) -> Self {
        // Encode as UTF-16BE with BOM
        let mut bytes = vec![0xFE, 0xFF]; // BOM
        for ch in text.encode_utf16() {
            bytes.extend_from_slice(&ch.to_be_bytes());
        }
        PdfHexString(bytes)
    }

    /// Returns the string data as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Decodes the hex string to text
    pub fn decode_text(&self) -> String {
        // Check for UTF-16BE BOM
        if self.0.len() >= 2 && self.0[0] == 0xFE && self.0[1] == 0xFF {
            let utf16: Vec<u16> = self.0[2..]
                .chunks(2)
                .filter_map(|chunk| {
                    if chunk.len() == 2 {
                        Some(u16::from_be_bytes([chunk[0], chunk[1]]))
                    } else {
                        None
                    }
                })
                .collect();
            String::from_utf16_lossy(&utf16)
        } else {
            String::from_utf8_lossy(&self.0).into_owned()
        }
    }

    /// Encodes the hex string for PDF output
    pub fn encode(&self) -> String {
        let mut result = String::with_capacity(self.0.len() * 2 + 2);
        result.push('<');
        for byte in &self.0 {
            result.push_str(&format!("{:02X}", byte));
        }
        result.push('>');
        result
    }
}

impl fmt::Display for PdfHexString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl From<Vec<u8>> for PdfHexString {
    fn from(bytes: Vec<u8>) -> Self {
        PdfHexString::new(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_string() {
        let s = PdfString::from_str("Hello World");
        assert_eq!(s.encode(), "(Hello World)");
    }

    #[test]
    fn test_literal_string_escaping() {
        let s = PdfString::from_str("Hello\nWorld");
        assert_eq!(s.encode(), "(Hello\\nWorld)");

        let s = PdfString::from_str("Test (parentheses)");
        assert_eq!(s.encode(), "(Test \\(parentheses\\))");
    }

    #[test]
    fn test_hex_string() {
        let s = PdfHexString::new(vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]);
        assert_eq!(s.encode(), "<48656C6C6F>");
    }

    #[test]
    fn test_hex_string_from_text() {
        let s = PdfHexString::from_text("Hi");
        // UTF-16BE with BOM: FE FF 00 48 00 69
        assert_eq!(s.as_bytes(), &[0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69]);
    }

    #[test]
    fn test_decode_text() {
        let s = PdfHexString::from_text("Hello");
        assert_eq!(s.decode_text(), "Hello");
    }

    #[test]
    fn test_from_text_ascii() {
        // ASCII text should stay as-is
        let s = PdfString::from_text("Hello World");
        assert_eq!(s.as_bytes(), b"Hello World");
    }

    #[test]
    fn test_from_text_unicode() {
        // Non-ASCII text should be encoded as UTF-16BE with BOM
        let s = PdfString::from_text("你好");
        // Should start with BOM (0xFE 0xFF)
        assert_eq!(s.as_bytes()[0], 0xFE);
        assert_eq!(s.as_bytes()[1], 0xFF);
        // Decode should round-trip correctly
        assert_eq!(s.decode_text(), "你好");
    }

    #[test]
    fn test_from_text_mixed() {
        // Mixed ASCII and Unicode
        let s = PdfString::from_text("Hello 世界");
        // Contains non-ASCII, so should be UTF-16BE
        assert_eq!(s.as_bytes()[0], 0xFE);
        assert_eq!(s.as_bytes()[1], 0xFF);
        assert_eq!(s.decode_text(), "Hello 世界");
    }
}
