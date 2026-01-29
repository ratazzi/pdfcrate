//! Custom Font Embedding Demo
//!
//! Demonstrates pdfcrate's TrueType font embedding:
//! - Embedding multiple font styles (regular, bold, italic)
//! - Font size variations
//! - Text measurement
//! - Mixing standard and embedded fonts
//!
//! Run with: cargo run --example custom_font_demo --features fonts

use pdfcrate::prelude::{Document, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;
use std::fs;

// Run `./examples/download-fonts.sh` first
const FONT_PATH: &str = "examples/fonts/Roboto-Regular.ttf";
const FONT_BOLD_PATH: &str = "examples/fonts/Roboto-Bold.ttf";
const FONT_ITALIC_PATH: &str = "examples/fonts/Roboto-Italic.ttf";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("custom_font_demo.pdf", |doc| {
        doc.title("Custom Font Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: custom_font_demo.pdf");
    Ok(())
}

/// Adds the custom font embedding demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    // Header band (light gray, top-left: 0, 842)
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 842.0], page_width, 82.0);
    });

    // Embed fonts
    let font_regular = doc.embed_font(fs::read(FONT_PATH)?)?;
    let font_bold = doc.embed_font(fs::read(FONT_BOLD_PATH)?)?;
    let font_italic = doc.embed_font(fs::read(FONT_ITALIC_PATH)?)?;

    // Page title (using embedded font)
    doc.font(&font_bold).size(28.0);
    doc.text_at("Custom Font Embedding", [margin, 800.0]);

    doc.font(&font_regular).size(12.0);
    doc.text_at("TrueType fonts with full Unicode support", [margin, 778.0]);

    // Section 1: Font showcase
    let mut y = 700.0;

    doc.font("Helvetica").size(14.0);
    doc.text_at("Font Comparison:", [margin, y]);
    y -= 30.0;

    // Standard font
    doc.font("Helvetica").size(16.0);
    doc.text_at(
        "Helvetica (Standard): The quick brown fox jumps over the lazy dog.",
        [margin, y],
    );
    y -= 25.0;

    // Embedded fonts
    doc.font(&font_regular).size(16.0);
    doc.text_at(
        "Roboto Regular: The quick brown fox jumps over the lazy dog.",
        [margin, y],
    );
    y -= 25.0;

    doc.font(&font_bold).size(16.0);
    doc.text_at(
        "Roboto Bold: The quick brown fox jumps over the lazy dog.",
        [margin, y],
    );
    y -= 25.0;

    doc.font(&font_italic).size(16.0);
    doc.text_at(
        "Roboto Italic: The quick brown fox jumps over the lazy dog.",
        [margin, y],
    );
    y -= 45.0;

    // Section 2: Size variations
    doc.font("Helvetica").size(14.0);
    doc.text_at("Size Variations:", [margin, y]);
    y -= 25.0;

    for size in [10.0, 14.0, 18.0, 24.0, 32.0] {
        doc.font(&font_regular).size(size);
        doc.text_at(&format!("{}pt: Roboto Font", size as i32), [margin, y]);
        y -= size + 8.0;
    }
    y -= 20.0;

    // Section 3: Text measurement
    doc.font("Helvetica").size(14.0);
    doc.text_at("Text Measurement:", [margin, y]);
    y -= 30.0;

    doc.font(&font_regular).size(18.0);
    let sample_text = "Measured Text Width";
    let text_width = doc.measure_text(sample_text);

    // Draw the text
    doc.text_at(sample_text, [margin, y]);

    // Draw a line under it showing the measured width
    doc.stroke(|ctx| {
        ctx.color(0.9, 0.2, 0.2)
            .line_width(2.0)
            .line([margin, y - 5.0], [margin + text_width, y - 5.0]);
    });

    doc.font(&font_regular).size(11.0);
    doc.text_at(
        &format!("Width: {:.1} points at 18pt", text_width),
        [margin, y - 20.0],
    );
    y -= 60.0;

    // Section 4: Mixed content
    doc.font("Helvetica").size(14.0);
    doc.text_at("Mixed Fonts in Document:", [margin, y]);
    y -= 30.0;

    // Draw a box with mixed font content (using top-left origin)
    doc.stroke(|ctx| {
        ctx.gray(0.7).line_width(1.0).rounded_rectangle(
            [margin, y + 10.0],
            page_width - margin * 2.0,
            90.0,
            8.0,
        );
    });

    doc.font(&font_bold).size(14.0);
    doc.text_at("Note:", [margin + 15.0, y - 10.0]);

    doc.font(&font_regular).size(12.0);
    doc.text_at(
        "This PDF demonstrates seamless mixing of standard PDF fonts",
        [margin + 15.0, y - 30.0],
    );
    doc.text_at(
        "(Helvetica, Times, Courier) with embedded TrueType fonts (Roboto).",
        [margin + 15.0, y - 45.0],
    );
    doc.text_at(
        "Text is fully searchable and can be copied from the PDF.",
        [margin + 15.0, y - 60.0],
    );

    Ok(())
}
