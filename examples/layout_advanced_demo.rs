//! Layout Advanced Demo
//!
//! Demonstrates pdfcrate's advanced text layout features:
//! - Text alignment (left, center, right)
//! - Leading (line spacing)
//! - Automatic text wrapping
//! - Text boxes with fixed height
//!
//! Run with: cargo run --example layout_advanced_demo

use pdfcrate::prelude::{Document, LayoutDocument, Margin, PageLayout, PageSize, TextAlign};
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

    // Header (absolute positioning, same as Prawn canvas block)
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

    // Switch to LayoutDocument with Prawn-compatible margin: 36
    let doc_owned = std::mem::take(doc);
    let mut layout = LayoutDocument::with_margin(doc_owned, Margin::new(36.0, 36.0, 36.0, 36.0));

    // Prawn: move_cursor_to bounds.top - 82
    // bounds.top = page_height - 2*margin = 842 - 72 = 770
    let bounds_top = layout.bounds().height();
    layout.move_cursor_to(bounds_top - 82.0);

    layout.indent(12.0, 0.0, |l| {
        // Section 1: Text Alignment
        l.font("Helvetica-Bold").size(12.0);
        l.text("1. Text Alignment");
        l.move_down(8.0);

        l.font("Helvetica").size(9.0);
        l.align(TextAlign::Left);
        l.text("Left aligned text (default)");
        l.align(TextAlign::Center);
        l.text("Center aligned text");
        l.align(TextAlign::Right);
        l.text("Right aligned text");
        l.align(TextAlign::Left);
        l.move_down(10.0);

        // Section 2: Leading (Line Spacing)
        l.font("Helvetica-Bold").size(12.0);
        l.text("2. Leading (Line Spacing)");
        l.move_down(8.0);

        let col_width = (l.bounds().width() - 20.0) / 2.0;
        let col_top = l.cursor();

        // Left column - Default and Tight
        l.bounding_box([0.0, col_top], col_width, None, |l| {
            l.font("Helvetica").size(8.5);
            l.text("Default leading (1.2x):");
            l.font("Helvetica").size(8.0);
            // Prawn leading: 2 → multiplier = 1 + 2/font_height
            let fh = l.font_height();
            l.leading(1.0 + 2.0 / fh);
            l.text("  Line 1 with normal spacing");
            l.text("  Line 2 with normal spacing");
            l.leading(1.0);
            l.move_down(8.0);
            l.font("Helvetica").size(8.5);
            l.text("Tight leading (1.0x):");
            l.font("Helvetica").size(8.0);
            // Prawn leading: 0 → multiplier = 1.0
            l.text("  Line 1 with tight spacing");
            l.text("  Line 2 with tight spacing");
        });

        // Right column - Loose (positioned at cursor + 60 to align with left column top)
        l.bounding_box([col_width + 20.0, l.cursor() + 60.0], col_width, None, |l| {
            l.font("Helvetica").size(8.5);
            l.text("Loose leading (1.8x):");
            l.font("Helvetica").size(8.0);
            // Prawn leading: 6 → multiplier = 1 + 6/font_height
            let fh = l.font_height();
            l.leading(1.0 + 6.0 / fh);
            l.text("  Line 1 with loose spacing");
            l.text("  Line 2 with loose spacing");
            l.leading(1.0);
        });

        l.move_down(30.0);

        // Section 3: Automatic Text Wrapping
        l.font("Helvetica-Bold").size(12.0);
        l.text("3. Automatic Text Wrapping");
        l.move_down(8.0);

        l.font("Helvetica").size(8.5);
        l.text_wrap("This demonstrates automatic text wrapping. The text automatically wraps to fit within the available width, making it easy to create flowing paragraphs without manual line breaks.");
        l.move_down(10.0);

        // Section 4: Text Box (Fixed Height)
        l.font("Helvetica-Bold").size(12.0);
        l.text("4. Text Box (Fixed Height)");
        l.move_down(8.0);

        let box_width = (l.bounds().width() - 30.0) / 2.0;
        let box_height = 55.0;
        let box_top = l.cursor();

        // Left text box
        l.bounding_box([0.0, box_top], box_width, Some(box_height), |l| {
            l.stroke_bounds();
            l.font("Helvetica").size(7.5);
            l.text_wrap("Text boxes constrain content to a fixed height. Overflow is clipped. Useful for predictable layouts where text must fit within specific boundaries.");
        });

        // Right text box
        l.set_cursor(box_top);
        l.bounding_box([box_width + 30.0, box_top], box_width, Some(box_height), |l| {
            l.stroke_bounds();
            l.font("Helvetica").size(7.5);
            l.text_wrap("Second text box at the same vertical position. Each box can have different content while maintaining consistent structure.");
        });

        l.move_cursor_to(box_top - box_height - 20.0);

        // Footer
        l.font("Helvetica-Oblique").size(8.0);
        l.align(TextAlign::Center);
        l.text("All text layout features work seamlessly");
        l.align(TextAlign::Left);
    });

    *doc = layout.into_inner();
    Ok(())
}
