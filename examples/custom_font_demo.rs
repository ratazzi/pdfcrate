//! Custom Font Embedding Demo
//!
//! Demonstrates pdfcrate's TrueType font embedding:
//! - Embedding multiple font styles (regular, bold, italic)
//! - Font size variations
//! - Text measurement
//! - Mixing standard and embedded fonts
//!
//! Run with: cargo run --example custom_font_demo --features fonts

use pdfcrate::prelude::{Document, LayoutDocument, Margin, PageLayout, PageSize};
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
    let (page_width, page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    // Header band
    doc.fill(|ctx| {
        ctx.color("F2F2F2")
            .rectangle([0.0, page_height], page_width, 82.0);
    });

    let roboto = doc.embed_font(fs::read(FONT_PATH)?)?;
    let roboto_bold = doc.embed_font(fs::read(FONT_BOLD_PATH)?)?;
    let roboto_italic = doc.embed_font(fs::read(FONT_ITALIC_PATH)?)?;

    doc.font(&roboto_bold).size(28.0);
    doc.text_at("Custom Font Embedding", [margin, 800.0]);

    doc.font(&roboto).size(12.0);
    doc.text_at("TrueType fonts with full Unicode support", [margin, 778.0]);

    let doc_owned = std::mem::take(doc);
    let mut layout = LayoutDocument::with_margin(
        doc_owned,
        Margin::new(142.0, margin - 12.0, margin - 12.0, margin - 12.0),
    );

    let abs_bottom = layout.bounds().absolute_bottom();

    layout.indent(12.0, 0.0, |layout| {
        let bounds_left = layout.bounds().absolute_left();
        layout.font("Helvetica").size(14.0);
        layout.text("Font Comparison:");
        layout.move_down(16.0);

        layout.font("Helvetica").size(16.0);
        layout.text("Helvetica (Standard): The quick brown fox jumps over the lazy dog.");
        layout.move_down(6.0);

        layout.font(&roboto).size(16.0);
        layout.text("Roboto Regular: The quick brown fox jumps over the lazy dog.");
        layout.move_down(6.0);

        layout.font(&roboto_bold).size(16.0);
        layout.text("Roboto Bold: The quick brown fox jumps over the lazy dog.");
        layout.move_down(6.0);

        layout.font(&roboto_italic).size(16.0);
        layout.text("Roboto Italic: The quick brown fox jumps over the lazy dog.");
        layout.move_down(26.0);

        layout.font("Helvetica").size(14.0);
        layout.text("Size Variations:");
        layout.move_down(8.0);

        for size in [10.0, 14.0, 18.0, 24.0, 32.0] {
            layout.font(&roboto).size(size);
            layout.text(&format!("{}pt: Roboto Font", size as i32));
        }
        layout.move_down(20.0);

        // Text measurement
        layout.font("Helvetica").size(14.0);
        layout.text("Text Measurement:");
        layout.move_down(16.0);

        layout.font(&roboto).size(18.0);
        let sample_text = "Measured Text Width";
        let text_width = layout.measure_text(sample_text);
        layout.text(sample_text);

        let y = layout.cursor() + abs_bottom;
        layout.stroke(|ctx| {
            ctx.color("E63333").line_width(2.0).line(
                [bounds_left, y - 15.0],
                [bounds_left + text_width, y - 15.0],
            );
        });

        layout.font(&roboto).size(11.0);
        layout.text(&format!("Width: {:.1} points at 18pt", text_width));
        layout.move_down(26.0);

        // Mixed content
        layout.font("Helvetica").size(14.0);
        layout.text("Mixed Fonts in Document:");
        layout.move_down(16.0);

        let box_y = layout.cursor() + abs_bottom;

        layout.stroke(|ctx| {
            ctx.color("666666").line_width(1.0).rounded_rectangle(
                [bounds_left, box_y + 10.0],
                page_width - margin * 2.0,
                90.0,
                8.0,
            );
        });

        layout.font(&roboto_bold).size(14.0);
        layout.text_at("Note:", [bounds_left + 15.0, box_y - 10.0]);

        layout.font(&roboto).size(12.0);
        layout.text_at(
            "This PDF demonstrates seamless mixing of standard PDF fonts",
            [bounds_left + 15.0, box_y - 30.0],
        );
        layout.text_at(
            "(Helvetica, Times, Courier) with embedded TrueType fonts (Roboto).",
            [bounds_left + 15.0, box_y - 45.0],
        );
        layout.text_at(
            "Text is fully searchable and can be copied from the PDF.",
            [bounds_left + 15.0, box_y - 60.0],
        );
    });

    *doc = layout.into_inner();
    Ok(())
}
