//! PDF Name object
//!
//! Names are atomic symbols uniquely defined by a sequence of characters.
//! They start with a forward slash (/) and can contain any characters except
//! whitespace and delimiters.

use std::fmt;

/// A PDF Name object
///
/// Names are used as keys in dictionaries and as identifiers throughout PDF.
/// For example: /Type, /Page, /Font
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PdfName(String);

impl PdfName {
    /// Creates a new PdfName from a string
    ///
    /// The string should not include the leading slash.
    pub fn new<S: Into<String>>(name: S) -> Self {
        PdfName(name.into())
    }

    /// Returns the name as a string slice (without the leading slash)
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the raw bytes of the name
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Encodes the name for PDF output, escaping special characters
    pub fn encode(&self) -> String {
        let mut result = String::with_capacity(self.0.len() + 1);
        result.push('/');

        for byte in self.0.bytes() {
            // Characters that need to be encoded as #XX
            if byte < 0x21
                || byte > 0x7E
                || byte == b'#'
                || byte == b'/'
                || byte == b'%'
                || byte == b'('
                || byte == b')'
                || byte == b'<'
                || byte == b'>'
                || byte == b'['
                || byte == b']'
                || byte == b'{'
                || byte == b'}'
            {
                result.push('#');
                result.push_str(&format!("{:02X}", byte));
            } else {
                result.push(byte as char);
            }
        }

        result
    }

    /// Common PDF names
    pub const TYPE: PdfName = PdfName(String::new()); // Placeholder, will use lazy_static or const fn
}

// Common PDF names as constants
impl PdfName {
    /// Creates /Type name
    pub fn type_() -> Self {
        PdfName::new("Type")
    }

    /// Creates /Subtype name
    pub fn subtype() -> Self {
        PdfName::new("Subtype")
    }

    /// Creates /Page name
    pub fn page() -> Self {
        PdfName::new("Page")
    }

    /// Creates /Pages name
    pub fn pages() -> Self {
        PdfName::new("Pages")
    }

    /// Creates /Catalog name
    pub fn catalog() -> Self {
        PdfName::new("Catalog")
    }

    /// Creates /Font name
    pub fn font() -> Self {
        PdfName::new("Font")
    }

    /// Creates /Resources name
    pub fn resources() -> Self {
        PdfName::new("Resources")
    }

    /// Creates /Contents name
    pub fn contents() -> Self {
        PdfName::new("Contents")
    }

    /// Creates /MediaBox name
    pub fn media_box() -> Self {
        PdfName::new("MediaBox")
    }

    /// Creates /Parent name
    pub fn parent() -> Self {
        PdfName::new("Parent")
    }

    /// Creates /Kids name
    pub fn kids() -> Self {
        PdfName::new("Kids")
    }

    /// Creates /Count name
    pub fn count() -> Self {
        PdfName::new("Count")
    }

    /// Creates /Length name
    pub fn length() -> Self {
        PdfName::new("Length")
    }

    /// Creates /Filter name
    pub fn filter() -> Self {
        PdfName::new("Filter")
    }

    /// Creates /FlateDecode name
    pub fn flate_decode() -> Self {
        PdfName::new("FlateDecode")
    }
}

impl fmt::Display for PdfName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl From<&str> for PdfName {
    fn from(s: &str) -> Self {
        PdfName::new(s)
    }
}

impl From<String> for PdfName {
    fn from(s: String) -> Self {
        PdfName::new(s)
    }
}

impl AsRef<str> for PdfName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_creation() {
        let name = PdfName::new("Type");
        assert_eq!(name.as_str(), "Type");
    }

    #[test]
    fn test_name_display() {
        let name = PdfName::new("Type");
        assert_eq!(format!("{}", name), "/Type");
    }

    #[test]
    fn test_name_encoding() {
        let name = PdfName::new("Type");
        assert_eq!(name.encode(), "/Type");

        // Test special character encoding
        let name_with_space = PdfName::new("Name With Space");
        // Space (0x20) is encoded but 0x21 and above are not
        assert!(name_with_space.encode().contains("#20"));
    }

    #[test]
    fn test_common_names() {
        assert_eq!(PdfName::type_().as_str(), "Type");
        assert_eq!(PdfName::page().as_str(), "Page");
        assert_eq!(PdfName::catalog().as_str(), "Catalog");
    }

    #[test]
    fn test_whitespace_encoding() {
        // All whitespace characters should be encoded as #XX
        // NULL (0x00), TAB (0x09), LF (0x0A), FF (0x0C), CR (0x0D), SPACE (0x20)
        let name = PdfName::new("a\0b\tc\nd\x0Ce\rf g");
        let encoded = name.encode();

        assert!(encoded.contains("#00"), "NULL should be encoded");
        assert!(encoded.contains("#09"), "TAB should be encoded");
        assert!(encoded.contains("#0A"), "LF should be encoded");
        assert!(encoded.contains("#0C"), "FF should be encoded");
        assert!(encoded.contains("#0D"), "CR should be encoded");
        assert!(encoded.contains("#20"), "SPACE should be encoded");

        // Should not contain raw whitespace
        assert!(!encoded.contains('\t'), "TAB should not appear raw");
        assert!(!encoded.contains('\n'), "LF should not appear raw");
        assert!(
            !encoded.contains(' '),
            "SPACE should not appear raw (except in #20)"
        );
    }
}
