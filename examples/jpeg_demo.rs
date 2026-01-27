//! JPEG Image Demo
//!
//! Demonstrates pdfcrate's JPEG image embedding:
//! - Converting PNG to JPEG at runtime
//! - Embedding JPEG in PDF
//! - Fitting image within page bounds while maintaining aspect ratio
//!
//! Run with: cargo run --example jpeg_demo

use image::codecs::jpeg::JpegEncoder;
use image::ImageReader;
use pdfcrate::image::embed_jpeg;
use pdfcrate::prelude::{Document, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load PNG and convert to JPEG
    let img = ImageReader::open("examples/example.png")?.decode()?;
    let rgb_img = img.to_rgb8();

    let mut jpeg_bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, 85);
    encoder.encode_image(&rgb_img)?;

    let jpeg_info = embed_jpeg(&jpeg_bytes)?;
    let (width, height) = (jpeg_info.width, jpeg_info.height);

    Document::generate("jpeg_demo.pdf", |doc| {
        doc.title("JPEG Image Demo").author("pdfcrate");
        add_page(doc, &jpeg_bytes, width, height)?;
        Ok(())
    })?;

    println!("Created: jpeg_demo.pdf");
    Ok(())
}

/// Fit image within page bounds while maintaining aspect ratio
fn fit_image(
    page_width: f64,
    page_height: f64,
    margin: f64,
    header_height: f64,
    img_width: u32,
    img_height: u32,
) -> (f64, f64, f64, f64) {
    let available_width = page_width - 2.0 * margin;
    let available_height = page_height - 2.0 * margin - header_height;

    let scale_x = available_width / img_width as f64;
    let scale_y = available_height / img_height as f64;
    let scale = scale_x.min(scale_y);

    let draw_width = img_width as f64 * scale;
    let draw_height = img_height as f64 * scale;

    let draw_x = margin + (available_width - draw_width) / 2.0;
    let draw_y = margin + (available_height - draw_height) / 2.0;

    (draw_x, draw_y, draw_width, draw_height)
}

/// Adds the JPEG image demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document, jpeg_bytes: &[u8], width: u32, height: u32) -> PdfResult<()> {
    let (page_width, page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 36.0;
    let header_height = 32.0;

    let (draw_x, draw_y, draw_width, draw_height) = fit_image(
        page_width,
        page_height,
        margin,
        header_height,
        width,
        height,
    );

    doc.font("Helvetica").size(14.0);
    doc.text_at(
        "JPEG (converted from PNG at runtime)",
        [margin, page_height - margin - 16.0],
    );
    doc.image_jpeg(jpeg_bytes, [draw_x, draw_y], draw_width, draw_height)?;

    Ok(())
}
