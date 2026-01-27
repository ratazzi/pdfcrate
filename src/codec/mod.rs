//! Stream encoding/decoding
//!
//! This module handles compression and encoding of PDF streams.

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};

use crate::error::Error;
use crate::Result;

/// Compresses data using zlib/deflate
pub fn flate_encode(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .map_err(|e| Error::Compression(e.to_string()))?;
    encoder
        .finish()
        .map_err(|e| Error::Compression(e.to_string()))
}

/// Decompresses zlib/deflate data
pub fn flate_decode(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decoded = Vec::new();
    decoder
        .read_to_end(&mut decoded)
        .map_err(|e| Error::Compression(e.to_string()))?;
    Ok(decoded)
}

/// Encodes data as ASCII85
pub fn ascii85_encode(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len() * 5 / 4 + 10);
    result.extend_from_slice(b"<~");

    for chunk in data.chunks(4) {
        let mut value: u32 = 0;
        for (i, &byte) in chunk.iter().enumerate() {
            value |= (byte as u32) << (24 - i * 8);
        }

        if chunk.len() == 4 && value == 0 {
            result.push(b'z');
        } else {
            let mut encoded = [0u8; 5];
            for i in (0..5).rev() {
                encoded[i] = (value % 85) as u8 + 33;
                value /= 85;
            }
            result.extend_from_slice(&encoded[..chunk.len() + 1]);
        }
    }

    result.extend_from_slice(b"~>");
    result
}

/// Decodes ASCII85 data
pub fn ascii85_decode(data: &[u8]) -> Result<Vec<u8>> {
    // Strip delimiters
    let data = if data.starts_with(b"<~") && data.ends_with(b"~>") {
        &data[2..data.len() - 2]
    } else {
        data
    };

    let mut result = Vec::new();
    let mut chunk = [0u32; 5];
    let mut count = 0;

    for &byte in data {
        match byte {
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' => continue, // Skip whitespace
            b'z' => {
                if count != 0 {
                    return Err(Error::Encoding("Invalid 'z' in ASCII85".to_string()));
                }
                result.extend_from_slice(&[0, 0, 0, 0]);
            }
            33..=117 => {
                chunk[count] = (byte - 33) as u32;
                count += 1;

                if count == 5 {
                    let value = chunk[0] * 85 * 85 * 85 * 85
                        + chunk[1] * 85 * 85 * 85
                        + chunk[2] * 85 * 85
                        + chunk[3] * 85
                        + chunk[4];
                    result.extend_from_slice(&value.to_be_bytes());
                    count = 0;
                }
            }
            _ => return Err(Error::Encoding(format!("Invalid ASCII85 byte: {}", byte))),
        }
    }

    // Handle partial group
    if count > 0 {
        // Pad with 'u' (84)
        for item in chunk.iter_mut().skip(count) {
            *item = 84;
        }
        let value = chunk[0] * 85 * 85 * 85 * 85
            + chunk[1] * 85 * 85 * 85
            + chunk[2] * 85 * 85
            + chunk[3] * 85
            + chunk[4];
        let bytes = value.to_be_bytes();
        result.extend_from_slice(&bytes[..count - 1]);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flate_roundtrip() {
        let data = b"Hello World! This is a test of compression.";
        let compressed = flate_encode(data).unwrap();
        let decompressed = flate_decode(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_ascii85_encode() {
        let data = b"test";
        let encoded = ascii85_encode(data);
        assert!(encoded.starts_with(b"<~"));
        assert!(encoded.ends_with(b"~>"));
    }

    #[test]
    fn test_ascii85_roundtrip() {
        let data = b"Hello World!";
        let encoded = ascii85_encode(data);
        let decoded = ascii85_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
