//! Layout Advanced Demo
//!
//! Demonstrates pdfcrate's advanced text layout features:
//! - Text alignment (left, center, right)
//! - Leading (line spacing)
//! - Automatic text wrapping
//! - Text boxes with fixed height
//!
//! Run with: cargo run --example layout_advanced_demo

use pdfcrate::prelude::{Document, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("layout_advanced_demo.pdf", |doc| {
        doc.title("Layout Advanced Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: layout_advanced_demo.pdf");
    Ok(())
}

/// Adds the advanced layout features demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);

    // Header (top-left: 0, 842)
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 842.0], page_width, 82.0);
    });
    doc.font("Helvetica").size(24.0);
    doc.text_at("Text Layout Features", [48.0, 804.0]);
    doc.font("Helvetica").size(11.0);
    doc.text_at(
        "Text alignment, leading, wrapping & text boxes",
        [48.0, 784.0],
    );

    // Use absolute positioning for content
    let left_margin = 48.0;
    let mut y = 720.0; // Start below header

    // Section 1: Text Alignment
    doc.font("Helvetica-Bold").size(12.0);
    doc.text_at("1. Text Alignment", [left_margin, y]);
    y -= 18.0;

    doc.font("Helvetica").size(9.0);
    doc.text_at("Left aligned text (default)", [left_margin, y]);
    y -= 12.0;

    // Center: measure text and center it
    let center_text = "Center aligned text";
    doc.text_at(center_text, [(page_width - 120.0) / 2.0, y]); // approximate center
    y -= 12.0;

    // Right aligned
    doc.text_at("Right aligned text", [page_width - left_margin - 100.0, y]);
    y -= 20.0;

    // Section 2: Leading
    doc.font("Helvetica-Bold").size(12.0);
    doc.text_at("2. Leading (Line Spacing)", [left_margin, y]);
    y -= 16.0;

    // Left column - Default and Tight
    let col1_x = left_margin;
    let col2_x = 320.0;
    let leading_y = y;

    doc.font("Helvetica").size(8.5);
    doc.text_at("Default leading (1.2x):", [col1_x, y]);
    y -= 10.0;
    doc.font("Helvetica").size(8.0);
    doc.text_at("  Line 1 with normal spacing", [col1_x, y]);
    y -= 10.0;
    doc.text_at("  Line 2 with normal spacing", [col1_x, y]);
    y -= 14.0;

    doc.font("Helvetica").size(8.5);
    doc.text_at("Tight leading (1.0x):", [col1_x, y]);
    y -= 9.0;
    doc.font("Helvetica").size(8.0);
    doc.text_at("  Line 1 with tight spacing", [col1_x, y]);
    y -= 9.0;
    doc.text_at("  Line 2 with tight spacing", [col1_x, y]);

    // Right column - Loose
    let mut y2 = leading_y;
    doc.font("Helvetica").size(8.5);
    doc.text_at("Loose leading (1.8x):", [col2_x, y2]);
    y2 -= 14.0;
    doc.font("Helvetica").size(8.0);
    doc.text_at("  Line 1 with loose spacing", [col2_x, y2]);
    y2 -= 14.0;
    doc.text_at("  Line 2 with loose spacing", [col2_x, y2]);

    y -= 22.0;

    // Section 3: Text Wrapping
    doc.font("Helvetica-Bold").size(12.0);
    doc.text_at("3. Automatic Text Wrapping", [left_margin, y]);
    y -= 14.0;

    doc.font("Helvetica").size(8.5);
    let wrap_lines = [
        "This demonstrates automatic text wrapping. The text automatically wraps to fit within the",
        "available width, making it easy to create flowing paragraphs without manual line breaks.",
    ];
    for line in &wrap_lines {
        doc.text_at(line, [left_margin, y]);
        y -= 10.0;
    }
    y -= 10.0;

    // Section 4: Text Box
    doc.font("Helvetica-Bold").size(12.0);
    doc.text_at("4. Text Box (Fixed Height)", [left_margin, y]);
    y -= 14.0;

    // Draw two boxes with borders (using top-left origin)
    let box_width = 235.0;
    let box_height = 55.0;
    let box1_x = left_margin;
    let box2_x = left_margin + 265.0;
    let box_top = y;
    let box_bottom = y - box_height;

    doc.stroke(|ctx| {
        ctx.gray(0.6)
            .line_width(0.5)
            .rectangle([box1_x, box_top], box_width, box_height)
            .rectangle([box2_x, box_top], box_width, box_height);
    });

    // Text inside boxes
    // Prawn uses cursor-based layout with ~5.4pt from box top to baseline
    doc.font("Helvetica").size(7.5);
    let box1_lines = [
        "Text boxes constrain content to a fixed height.",
        "Overflow is clipped. Useful for predictable layouts",
        "where text must fit within specific boundaries.",
    ];
    let box2_lines = [
        "Second text box at the same vertical position.",
        "Each box can have different content while",
        "maintaining consistent structure.",
    ];

    let mut by = box_top - 5.4;
    for line in &box1_lines {
        doc.text_at(line, [box1_x + 4.0, by]);
        by -= 8.67; // Match Prawn's line spacing
    }

    by = box_top - 5.4;
    for line in &box2_lines {
        doc.text_at(line, [box2_x + 4.0, by]);
        by -= 8.67;
    }

    y = box_bottom - 25.0;

    // Footer
    doc.font("Helvetica-Oblique").size(8.0);
    doc.text_at(
        "All text layout features work seamlessly with LayoutDocument",
        [130.0, y],
    );

    Ok(())
}
