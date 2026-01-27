//! PNG Alpha Transparency Demo
//!
//! Demonstrates pdfcrate's PNG alpha channel support:
//! - Loading PNG with alpha transparency
//! - Rendering transparent PNG over colored background
//! - Alpha blending with background colors
//!
//! Run with: cargo run --example alpha_demo

use pdfcrate::prelude::{Document, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let alpha_bytes = fs::read("examples/example.png")?;
    let (width, height) = read_png_dimensions(&alpha_bytes)?;

    Document::generate("alpha_demo.pdf", |doc| {
        doc.title("PNG Alpha Demo").author("pdfcrate");
        add_page(doc, &alpha_bytes, width, height)?;
        Ok(())
    })?;

    println!("Created: alpha_demo.pdf");
    Ok(())
}

/// Read PNG dimensions from bytes
fn read_png_dimensions(bytes: &[u8]) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    if bytes.len() < 24 {
        return Err("PNG too small".into());
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Ok((width, height))
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

/// Adds the PNG alpha transparency demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document, alpha_bytes: &[u8], width: u32, height: u32) -> PdfResult<()> {
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
        "PNG with alpha transparency",
        [margin, page_height - margin - 16.0],
    );
    doc.image_png(alpha_bytes, [draw_x, draw_y], draw_width, draw_height)?;

    Ok(())
}
