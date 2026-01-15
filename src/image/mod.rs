//! PDF Image handling
//!
//! This module handles embedding images in PDF documents.

use crate::error::{Error, Result};
use crate::objects::{PdfName, PdfObject, PdfStream};

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
}

/// PNG magic bytes (signature)
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// JPEG magic bytes (SOI marker + start of next marker)
const JPEG_SIGNATURE: [u8; 3] = [0xFF, 0xD8, 0xFF];

/// Detects the image format from raw bytes by checking magic bytes
///
/// Returns `Some(ImageFormat)` if recognized, `None` otherwise.
pub fn detect_format(data: &[u8]) -> Option<ImageFormat> {
    if data.len() >= 3 && data[0..3] == JPEG_SIGNATURE {
        Some(ImageFormat::Jpeg)
    } else if data.len() >= 8 && data[0..8] == PNG_SIGNATURE {
        Some(ImageFormat::Png)
    } else {
        None
    }
}

/// Embeds an image by auto-detecting the format
///
/// Supports JPEG and PNG formats. PNG requires the `png` feature.
pub fn embed_image(data: &[u8]) -> Result<ImageData> {
    match detect_format(data) {
        Some(ImageFormat::Jpeg) => embed_jpeg(data),
        #[cfg(feature = "png")]
        Some(ImageFormat::Png) => embed_png(data),
        #[cfg(not(feature = "png"))]
        Some(ImageFormat::Png) => Err(Error::Image(
            "PNG support requires the 'png' feature".to_string(),
        )),
        None => Err(Error::Image(
            "Unrecognized image format (expected JPEG or PNG)".to_string(),
        )),
    }
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
        if self.compressed {
            // JPEG uses DCTDecode
            let mut stream = PdfStream::from_data(self.data.clone());
            let stream_dict = stream.dict_mut();
            stream_dict.set("Type", PdfObject::Name(PdfName::new("XObject")));
            stream_dict.set("Subtype", PdfObject::Name(PdfName::new("Image")));
            stream_dict.set("Width", PdfObject::Integer(self.width as i64));
            stream_dict.set("Height", PdfObject::Integer(self.height as i64));
            stream_dict.set(
                "ColorSpace",
                PdfObject::Name(PdfName::new(self.color_space.pdf_name())),
            );
            stream_dict.set(
                "BitsPerComponent",
                PdfObject::Integer(self.bits_per_component as i64),
            );
            stream_dict.set("Filter", PdfObject::Name(PdfName::new("DCTDecode")));
            stream
        } else {
            // Compress with Flate while preserving image dictionary entries.
            let mut stream = PdfStream::from_data_compressed(self.data.clone());
            let stream_dict = stream.dict_mut();
            stream_dict.set("Type", PdfObject::Name(PdfName::new("XObject")));
            stream_dict.set("Subtype", PdfObject::Name(PdfName::new("Image")));
            stream_dict.set("Width", PdfObject::Integer(self.width as i64));
            stream_dict.set("Height", PdfObject::Integer(self.height as i64));
            stream_dict.set(
                "ColorSpace",
                PdfObject::Name(PdfName::new(self.color_space.pdf_name())),
            );
            stream_dict.set(
                "BitsPerComponent",
                PdfObject::Integer(self.bits_per_component as i64),
            );
            stream
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

    #[test]
    fn test_detect_format_jpeg() {
        // JPEG signature: FF D8 FF
        let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(detect_format(&jpeg_data), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn test_detect_format_png() {
        // PNG signature: 89 50 4E 47 0D 0A 1A 0A
        let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00];
        assert_eq!(detect_format(&png_data), Some(ImageFormat::Png));
    }

    #[test]
    fn test_detect_format_unknown() {
        let unknown_data = [0x00, 0x01, 0x02, 0x03];
        assert_eq!(detect_format(&unknown_data), None);
    }

    #[test]
    fn test_detect_format_too_short() {
        let short_data = [0xFF, 0xD8];
        assert_eq!(detect_format(&short_data), None);
    }
}
