//! PDF Image handling
//!
//! This module handles embedding images in PDF documents.

use crate::error::{Error, Result};
use crate::objects::{PdfDict, PdfName, PdfObject, PdfStream};

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
}

/// Image data for embedding
pub struct ImageData {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Bits per component (usually 8)
    pub bits_per_component: u8,
    /// Color space name
    pub color_space: ColorSpace,
    /// The raw image data
    pub data: Vec<u8>,
    /// Optional soft mask (alpha channel)
    pub soft_mask: Option<Vec<u8>>,
    /// Whether data is already compressed
    pub compressed: bool,
}

/// Color space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    DeviceGray,
    DeviceRGB,
    DeviceCMYK,
}

impl ColorSpace {
    /// Returns the PDF name for this color space
    pub fn pdf_name(&self) -> &'static str {
        match self {
            ColorSpace::DeviceGray => "DeviceGray",
            ColorSpace::DeviceRGB => "DeviceRGB",
            ColorSpace::DeviceCMYK => "DeviceCMYK",
        }
    }

    /// Returns the number of components
    pub fn components(&self) -> u8 {
        match self {
            ColorSpace::DeviceGray => 1,
            ColorSpace::DeviceRGB => 3,
            ColorSpace::DeviceCMYK => 4,
        }
    }
}

/// Embeds a JPEG image
///
/// JPEG images can be embedded directly without decoding.
pub fn embed_jpeg(data: &[u8]) -> Result<ImageData> {
    // Parse JPEG header to get dimensions
    let (width, height, components) = parse_jpeg_header(data)?;

    let color_space = match components {
        1 => ColorSpace::DeviceGray,
        3 => ColorSpace::DeviceRGB,
        4 => ColorSpace::DeviceCMYK,
        _ => {
            return Err(Error::Image(format!(
                "Unsupported JPEG components: {}",
                components
            )))
        }
    };

    Ok(ImageData {
        width,
        height,
        bits_per_component: 8,
        color_space,
        data: data.to_vec(),
        soft_mask: None,
        compressed: true, // JPEG is DCT compressed
    })
}

/// Parses JPEG header to extract dimensions and components
fn parse_jpeg_header(data: &[u8]) -> Result<(u32, u32, u8)> {
    if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
        return Err(Error::Image("Invalid JPEG: missing SOI marker".to_string()));
    }

    let mut pos = 2;
    while pos < data.len() - 1 {
        if data[pos] != 0xFF {
            pos += 1;
            continue;
        }

        let marker = data[pos + 1];
        pos += 2;

        // Skip padding bytes
        if marker == 0xFF || marker == 0x00 {
            continue;
        }

        // SOF markers (Start of Frame)
        if matches!(marker, 0xC0..=0xCF) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
            if pos + 7 > data.len() {
                return Err(Error::Image("Invalid JPEG: truncated SOF".to_string()));
            }

            let height = u16::from_be_bytes([data[pos + 3], data[pos + 4]]) as u32;
            let width = u16::from_be_bytes([data[pos + 5], data[pos + 6]]) as u32;
            let components = data[pos + 7];

            return Ok((width, height, components));
        }

        // Skip segment
        if pos + 2 > data.len() {
            break;
        }
        let length = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
        pos += length;
    }

    Err(Error::Image(
        "Invalid JPEG: no SOF marker found".to_string(),
    ))
}

#[cfg(feature = "png")]
/// Embeds a PNG image
pub fn embed_png(data: &[u8]) -> Result<ImageData> {
    use png::Decoder;
    use std::io::Cursor;

    let decoder = Decoder::new(Cursor::new(data));
    let mut reader = decoder
        .read_info()
        .map_err(|e| Error::Image(e.to_string()))?;

    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| Error::Image(e.to_string()))?;

    let (color_space, has_alpha) = match info.color_type {
        png::ColorType::Grayscale => (ColorSpace::DeviceGray, false),
        png::ColorType::GrayscaleAlpha => (ColorSpace::DeviceGray, true),
        png::ColorType::Rgb => (ColorSpace::DeviceRGB, false),
        png::ColorType::Rgba => (ColorSpace::DeviceRGB, true),
        _ => return Err(Error::Image("Unsupported PNG color type".to_string())),
    };

    let components = color_space.components() as usize;
    let pixel_size = if has_alpha {
        components + 1
    } else {
        components
    };

    // Separate alpha channel if present
    let (image_data, soft_mask) = if has_alpha {
        let pixel_count = (info.width * info.height) as usize;
        let mut image = Vec::with_capacity(pixel_count * components);
        let mut mask = Vec::with_capacity(pixel_count);

        for pixel in buf[..pixel_count * pixel_size].chunks(pixel_size) {
            image.extend_from_slice(&pixel[..components]);
            mask.push(pixel[components]);
        }

        (image, Some(mask))
    } else {
        (
            buf[..info.width as usize * info.height as usize * components].to_vec(),
            None,
        )
    };

    Ok(ImageData {
        width: info.width,
        height: info.height,
        bits_per_component: info.bit_depth as u8,
        color_space,
        data: image_data,
        soft_mask,
        compressed: false,
    })
}

impl ImageData {
    /// Creates a PDF XObject stream for this image
    pub fn to_xobject(&self) -> PdfStream {
        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::new("XObject")));
        dict.set("Subtype", PdfObject::Name(PdfName::new("Image")));
        dict.set("Width", PdfObject::Integer(self.width as i64));
        dict.set("Height", PdfObject::Integer(self.height as i64));
        dict.set(
            "ColorSpace",
            PdfObject::Name(PdfName::new(self.color_space.pdf_name())),
        );
        dict.set(
            "BitsPerComponent",
            PdfObject::Integer(self.bits_per_component as i64),
        );

        if self.compressed {
            // JPEG uses DCTDecode
            dict.set("Filter", PdfObject::Name(PdfName::new("DCTDecode")));
            PdfStream::new(dict, self.data.clone())
        } else {
            // Compress with Flate
            PdfStream::from_data_compressed(self.data.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_space() {
        assert_eq!(ColorSpace::DeviceGray.pdf_name(), "DeviceGray");
        assert_eq!(ColorSpace::DeviceRGB.components(), 3);
    }

    // JPEG parsing tests would require sample JPEG data
}
