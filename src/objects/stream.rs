//! PDF Stream object
//!
//! Streams consist of a dictionary followed by a sequence of bytes.
//! They are used for page contents, images, fonts, and other binary data.

use std::fmt;

use super::{PdfDict, PdfName, PdfObject};

/// A PDF stream object
///
/// Streams are used to represent large or binary data in PDF.
/// They consist of a dictionary that describes the data, followed
/// by the raw bytes enclosed between `stream` and `endstream` keywords.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfStream {
    /// The stream dictionary containing metadata
    pub dict: PdfDict,
    /// The raw (possibly encoded) data
    pub data: Vec<u8>,
    /// Whether the data is already encoded
    encoded: bool,
}

impl PdfStream {
    /// Creates a new stream with the given dictionary and data
    pub fn new(dict: PdfDict, data: Vec<u8>) -> Self {
        PdfStream {
            dict,
            data,
            encoded: false,
        }
    }

    /// Creates a new stream with data and default dictionary
    pub fn from_data(data: Vec<u8>) -> Self {
        let mut dict = PdfDict::new();
        dict.set("Length", PdfObject::Integer(data.len() as i64));
        PdfStream {
            dict,
            data,
            encoded: false,
        }
    }

    /// Creates a new stream with compressed data
    pub fn from_data_compressed(data: Vec<u8>) -> Self {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data).expect("compression failed");
        let compressed = encoder.finish().expect("compression failed");

        let mut dict = PdfDict::new();
        dict.set("Length", PdfObject::Integer(compressed.len() as i64));
        dict.set("Filter", PdfObject::Name(PdfName::flate_decode()));

        PdfStream {
            dict,
            data: compressed,
            encoded: true,
        }
    }

    /// Returns the stream dictionary
    pub fn dict(&self) -> &PdfDict {
        &self.dict
    }

    /// Returns a mutable reference to the stream dictionary
    pub fn dict_mut(&mut self) -> &mut PdfDict {
        &mut self.dict
    }

    /// Returns the raw stream data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the length of the encoded data
    pub fn encoded_length(&self) -> usize {
        self.data.len()
    }

    /// Decodes the stream data if it's compressed
    pub fn decode(&self) -> crate::Result<Vec<u8>> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;

        let filter = self.dict.get("Filter");

        match filter {
            Some(PdfObject::Name(name)) if name.as_str() == "FlateDecode" => {
                let mut decoder = ZlibDecoder::new(&self.data[..]);
                let mut decoded = Vec::new();
                decoder
                    .read_to_end(&mut decoded)
                    .map_err(|e| crate::Error::Compression(e.to_string()))?;
                Ok(decoded)
            }
            Some(PdfObject::Name(name)) => {
                Err(crate::Error::Unsupported(format!(
                    "Filter {} not supported",
                    name
                )))
            }
            None => Ok(self.data.clone()),
            _ => Ok(self.data.clone()),
        }
    }

    /// Updates the Length entry in the dictionary
    pub fn update_length(&mut self) {
        self.dict.set("Length", PdfObject::Integer(self.data.len() as i64));
    }

    /// Sets the stream data and updates the length
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
        self.update_length();
    }

    /// Returns true if the stream has a filter applied
    pub fn is_filtered(&self) -> bool {
        self.dict.contains_key("Filter")
    }

    /// Gets the filter name if present
    pub fn filter(&self) -> Option<&PdfName> {
        self.dict.get_name("Filter")
    }
}

impl fmt::Display for PdfStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\nstream\n", self.dict)?;
        // Note: In actual PDF output, the raw bytes would be written here
        write!(f, "[{} bytes]", self.data.len())?;
        write!(f, "\nendstream")
    }
}

impl From<Vec<u8>> for PdfStream {
    fn from(data: Vec<u8>) -> Self {
        PdfStream::from_data(data)
    }
}

impl From<&[u8]> for PdfStream {
    fn from(data: &[u8]) -> Self {
        PdfStream::from_data(data.to_vec())
    }
}

impl From<String> for PdfStream {
    fn from(s: String) -> Self {
        PdfStream::from_data(s.into_bytes())
    }
}

impl From<&str> for PdfStream {
    fn from(s: &str) -> Self {
        PdfStream::from_data(s.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_creation() {
        let stream = PdfStream::from_data(b"Hello World".to_vec());
        assert_eq!(stream.data(), b"Hello World");
        assert_eq!(stream.encoded_length(), 11);
    }

    #[test]
    fn test_stream_length() {
        let stream = PdfStream::from_data(b"Test data".to_vec());
        let length = stream.dict().get_integer("Length");
        assert_eq!(length, Some(9));
    }

    #[test]
    fn test_stream_from_str() {
        let stream: PdfStream = "Test content".into();
        assert_eq!(stream.data(), b"Test content");
    }

    #[test]
    fn test_stream_compression() {
        let data = b"Hello World Hello World Hello World".to_vec();
        let stream = PdfStream::from_data_compressed(data.clone());

        // Compressed data should be smaller (usually)
        assert!(stream.is_filtered());
        assert_eq!(stream.filter().map(|n| n.as_str()), Some("FlateDecode"));

        // Decode and verify
        let decoded = stream.decode().unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_stream_decode_uncompressed() {
        let stream = PdfStream::from_data(b"Plain text".to_vec());
        let decoded = stream.decode().unwrap();
        assert_eq!(decoded, b"Plain text");
    }
}
