//! LayoutDocument Demo
//!
//! Demonstrates pdfcrate's Prawn-style cursor-based layout system:
//! - Nested bounding boxes
//! - Side-by-side layout
//! - Formatted text with mixed styles
//! - Cursor tracking
//! - Indentation
//! - Float positioning
//! - Bounds visualization
//!
//! Run with: cargo run --example layout_demo

use pdfcrate::api::{Color, TextFragment};
use pdfcrate::prelude::{Document, LayoutDocument, Margin, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("layout_demo.pdf", |doc| {
        doc.title("Layout Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: layout_demo.pdf");
    Ok(())
}

/// Adds the LayoutDocument demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _) = PageSize::A4.dimensions(PageLayout::Portrait);

    // Header band (top-left: 0, 842)
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 842.0], page_width, 82.0);
    });

    // Use 36pt margin to match Prawn default
    let margin = 36.0;

    doc.font("Helvetica").size(24.0);
    doc.text_at("Cursor Layout Demo", [margin, 800.0]);

    doc.font("Helvetica").size(11.0);
    doc.text_at(
        "Native cursor-based layout (no manual coordinate calculation)",
        [margin, 780.0],
    );

    // Create LayoutDocument wrapper
    // Match Prawn: bounds.top - 100 where bounds.top = 842 - 36 = 806
    // So cursor starts at 806 - 100 = 706, meaning top_margin = 842 - 706 = 136
    let doc_owned = std::mem::take(doc);
    let mut layout =
        LayoutDocument::with_margin(doc_owned, Margin::new(136.0, margin, margin, margin));

    // Section 1: Bounding Box Demo
    layout.font("Helvetica").size(12.0);
    layout.text("1. Nested Bounding Boxes:");
    layout.move_down(10.0);

    // Outer box (fixed height)
    // Prawn-style: pass cursor() as Y position
    let y = layout.cursor();
    layout.bounding_box([0.0, y], 220.0, Some(90.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(10.0);
        doc.text("Outer box (220x90)");
        doc.move_down(5.0);

        // Inner box (nested, stretchy)
        let inner_y = doc.cursor();
        doc.bounding_box([15.0, inner_y], 160.0, None, |doc| {
            doc.stroke_bounds();
            doc.font("Helvetica").size(9.0);
            doc.text("Inner nested box");
            doc.text("Auto-height (stretchy)");
        });
    });

    // Side-by-side boxes using float
    layout.move_down(15.0);
    layout.font("Helvetica").size(12.0);
    layout.text("2. Side-by-Side Layout:");
    layout.move_down(10.0);

    let box_top = layout.cursor();

    // Left box
    layout.bounding_box([0.0, box_top], 140.0, Some(60.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(9.0);
        doc.text("Left Box");
        doc.text("Width: 140pt");
    });

    // Right box (use move_cursor_to to position at same y level)
    layout.move_cursor_to(box_top);
    layout.bounding_box([160.0, box_top], 140.0, Some(60.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(9.0);
        doc.text("Right Box");
        doc.text("Offset: 160pt");
    });

    layout.move_down(15.0);

    // Section 2: Formatted text (mixed styles)
    layout.font("Helvetica").size(12.0);
    layout.text("3. Formatted Text (Mixed Styles):");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.formatted_text(&[
        TextFragment::new("This is "),
        TextFragment::new("bold").bold(),
        TextFragment::new(", "),
        TextFragment::new("italic").italic(),
        TextFragment::new(", and "),
        TextFragment::new("red").color(Color::RED),
        TextFragment::new(" text in one line."),
    ]);
    layout.formatted_text(&[
        TextFragment::new("Mixed: ").bold(),
        TextFragment::new("Times ").font("Times-Roman"),
        TextFragment::new("and "),
        TextFragment::new("Courier").font("Courier"),
        TextFragment::new(" fonts."),
    ]);

    layout.move_down(10.0);

    // Section 3: Cursor tracking
    layout.font("Helvetica").size(12.0);
    layout.text("4. Cursor Tracking:");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    let cursor1 = layout.cursor();
    layout.text(&format!("Cursor at: {:.1}pt", cursor1));
    layout.text("Each text() call auto-advances cursor");
    let cursor2 = layout.cursor();
    layout.text(&format!(
        "Now at: {:.1}pt (moved {:.1}pt)",
        cursor2,
        cursor1 - cursor2
    ));

    layout.move_down(15.0);

    // Section 4: Indent
    layout.font("Helvetica").size(12.0);
    layout.text("5. Indent:");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text("Normal margin.");
    layout.indent(30.0, 0.0, |l| {
        l.text("Indented 30pt from left.");
        l.indent(30.0, 0.0, |l| {
            l.text("Double indent (60pt total).");
        });
        l.text("Back to 30pt indent.");
    });
    layout.text("Back to normal.");

    layout.move_down(15.0);

    // Section 5: Float
    layout.font("Helvetica").size(12.0);
    layout.text("6. Float (temp position):");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text("Before float");
    layout.float(|l| {
        l.move_down(40.0);
        l.font("Helvetica").size(9.0);
        l.text(">> Floated 40pt down");
    });
    layout.text("After float (continues from 'Before')");
    layout.move_down(50.0);

    // Section 6: Bounds visualization (compact)
    layout.font("Helvetica").size(12.0);
    layout.text("7. Bounds Visualization:");
    layout.move_down(8.0);

    let y = layout.cursor();
    layout.bounding_box([0.0, y], 200.0, Some(50.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(9.0);
        doc.text("stroke_bounds() draws");
        doc.text("the current bounding box");
    });

    *doc = layout.into_inner();
    Ok(())
}
