//! PNG Image Demo
//!
//! Demonstrates pdfcrate's PNG image embedding:
//! - Loading PNG from file bytes
//! - Fitting image within page bounds while maintaining aspect ratio
//! - Centering image on page
//!
//! Run with: cargo run --example png_demo

use pdfcrate::prelude::{Document, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let png_bytes = fs::read("examples/example.png")?;
    let (width, height) = read_png_dimensions(&png_bytes)?;

    Document::generate("png_demo.pdf", |doc| {
        doc.title("PNG Image Demo").author("pdfcrate");
        add_page(doc, &png_bytes, width, height)?;
        Ok(())
    })?;

    println!("Created: png_demo.pdf");
    Ok(())
}

/// Read PNG dimensions from bytes
pub fn read_png_dimensions(bytes: &[u8]) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    // PNG header is 8 bytes, then IHDR chunk
    // IHDR chunk: 4 bytes length, 4 bytes "IHDR", 4 bytes width, 4 bytes height
    if bytes.len() < 24 {
        return Err("PNG too small".into());
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Ok((width, height))
}

/// Fit image within page bounds while maintaining aspect ratio
pub fn fit_image(
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

    // Center horizontally
    let draw_x = margin + (available_width - draw_width) / 2.0;
    // Position below header
    let draw_y = margin + (available_height - draw_height) / 2.0;

    (draw_x, draw_y, draw_width, draw_height)
}

/// Adds the PNG image demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document, png_bytes: &[u8], width: u32, height: u32) -> PdfResult<()> {
    let (page_width, page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 36.0;
    let bounds_width = page_width - 2.0 * margin;
    let bounds_height = page_height - 2.0 * margin;

    doc.font("Helvetica").size(14.0);
    doc.text_at("Embedded PNG", [margin, page_height - margin - 16.0]);

    // Fit image within [bounds_width, bounds_height - 50], matching Prawn behavior
    let fit_width = bounds_width;
    let fit_height = bounds_height - 50.0;
    let scale = (fit_width / width as f64).min(fit_height / height as f64);
    let draw_width = width as f64 * scale;
    let draw_height = height as f64 * scale;

    // Center in full bounds, matching Prawn's position: :center, vposition: :center
    let draw_x = margin + (bounds_width - draw_width) / 2.0;
    let draw_y = margin + (bounds_height - draw_height) / 2.0;

    doc.image_png(png_bytes, [draw_x, draw_y], draw_width, draw_height)?;

    Ok(())
}
