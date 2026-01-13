//! LayoutDocument Demo - Prawn-style cursor-based layout
//!
//! This example demonstrates the LayoutDocument API features:
//! - Automatic cursor management (no manual coordinate calculation)
//! - Headers and footers with RepeaterPages
//! - Automatic page numbering
//! - Nested bounding boxes
//! - Text wrapping and alignment
//! - Indentation and padding
//!
//! Run with: cargo run --example layout_demo --features "fonts"

use pdfcrate::prelude::*;
use std::error::Error;
use std::result::Result as StdResult;

fn main() -> StdResult<(), Box<dyn Error>> {
    let doc = Document::new();
    let mut layout = LayoutDocument::with_margin(doc, Margin::all(72.0)); // 1 inch margins

    // Configure page numbering
    layout.number_pages(
        PageNumberConfig::new("Page <page> of <total>")
            .position(PageNumberPosition::BottomCenter)
            .font_size(10.0),
    );

    // Add header to all pages
    layout.header(RepeaterPages::All, "LayoutDocument Demo");

    // Add footer with different content for odd/even pages
    layout.font("Helvetica").size(9.0);
    layout.repeat(RepeaterPages::Odd, "Odd Page Footer", [72.0, 30.0]);
    layout.repeat(RepeaterPages::Even, "Even Page Footer", [400.0, 30.0]);

    // === Page 1: Introduction ===
    layout.font("Helvetica").size(24.0);
    layout.text("LayoutDocument Features");
    layout.move_down(20.0);

    layout.font("Helvetica").size(12.0);
    layout.text("This PDF demonstrates the Prawn-style layout system.");
    layout.text("No manual coordinate calculations needed!");
    layout.move_down(15.0);

    layout.text("Features demonstrated:");
    layout.indent(20.0, 0.0, |l| {
        l.text("- Automatic cursor tracking");
        l.text("- Headers and footers");
        l.text("- Page numbering with format strings");
        l.text("- Nested bounding boxes");
        l.text("- Text wrapping and alignment");
        l.text("- Indentation and padding");
    });

    layout.move_down(30.0);

    // Demonstrate cursor tracking
    layout.font("Helvetica").size(14.0);
    layout.text("Cursor Tracking:");
    layout.move_down(10.0);

    layout.font("Helvetica").size(11.0);
    let cursor1 = layout.cursor();
    layout.text(&format!(
        "Current cursor: {:.1}pt from page bottom",
        cursor1
    ));
    layout.text("Each text() call automatically moves the cursor down.");
    let cursor2 = layout.cursor();
    layout.text(&format!(
        "Cursor moved {:.1}pt (line height)",
        cursor1 - cursor2
    ));

    // === Page 2: Bounding Boxes ===
    layout.start_new_page();

    layout.font("Helvetica").size(20.0);
    layout.text("Bounding Boxes");
    layout.move_down(15.0);

    layout.font("Helvetica").size(12.0);
    layout.text("Bounding boxes create isolated layout regions:");
    layout.move_down(15.0);

    // Fixed-height box
    layout.font("Helvetica").size(11.0);
    layout.text("Fixed-height box (200x100pt):");
    layout.move_down(5.0);

    layout.bounding_box([0.0, 0.0], 200.0, Some(100.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(10.0);
        doc.text("Content inside box");
        doc.text("Cursor is local to this box");
        doc.text("Box has fixed height");
    });

    layout.move_down(20.0);

    // Stretchy box
    layout.text("Stretchy box (auto-height):");
    layout.move_down(5.0);

    layout.bounding_box([0.0, 0.0], 200.0, None, |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(10.0);
        doc.text("This box grows");
        doc.text("to fit its content");
        doc.text("automatically");
    });

    layout.move_down(20.0);

    // Nested boxes
    layout.text("Nested bounding boxes:");
    layout.move_down(5.0);

    layout.bounding_box([0.0, 0.0], 300.0, Some(150.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(10.0);
        doc.text("Outer box (300x150)");

        doc.bounding_box([20.0, 0.0], 150.0, Some(80.0), |doc| {
            doc.stroke_bounds();
            doc.text("Inner box (150x80)");
            doc.text("Nested inside outer");
        });
    });

    // === Page 3: Text Features ===
    layout.start_new_page();

    layout.font("Helvetica").size(20.0);
    layout.text("Text Features");
    layout.move_down(15.0);

    // Text alignment
    layout.font("Helvetica").size(14.0);
    layout.text("Text Alignment:");
    layout.move_down(10.0);

    layout.font("Helvetica").size(11.0);

    layout.bounding_box([0.0, 0.0], 300.0, None, |doc| {
        doc.stroke_bounds();

        doc.align(TextAlign::Left);
        doc.text("Left aligned text");

        doc.align(TextAlign::Center);
        doc.text("Center aligned text");

        doc.align(TextAlign::Right);
        doc.text("Right aligned text");

        doc.align(TextAlign::Left); // Reset
    });

    layout.move_down(20.0);

    // Text wrapping
    layout.font("Helvetica").size(14.0);
    layout.text("Text Wrapping:");
    layout.move_down(10.0);

    layout.font("Helvetica").size(11.0);
    let long_text = "This is a long paragraph that will be automatically wrapped to fit within the bounding box width. The text wrapping respects word boundaries and creates multiple lines as needed.";

    layout.bounding_box([0.0, 0.0], 250.0, None, |doc| {
        doc.stroke_bounds();
        doc.text_wrap(long_text);
    });

    layout.move_down(20.0);

    // Indentation
    layout.font("Helvetica").size(14.0);
    layout.text("Indentation:");
    layout.move_down(10.0);

    layout.font("Helvetica").size(11.0);
    layout.text("Normal text at margin.");

    layout.indent(30.0, 0.0, |l| {
        l.text("Indented 30pt from left.");

        l.indent(30.0, 0.0, |l| {
            l.text("Double indent (60pt total).");
        });

        l.text("Back to single indent.");
    });

    layout.text("Back to normal margin.");

    // === Page 4: Layout Helpers ===
    layout.start_new_page();

    layout.font("Helvetica").size(20.0);
    layout.text("Layout Helpers");
    layout.move_down(15.0);

    // Float
    layout.font("Helvetica").size(14.0);
    layout.text("Float (temporary position):");
    layout.move_down(10.0);

    layout.font("Helvetica").size(11.0);
    layout.text("Before float");

    layout.float(|l| {
        l.move_down(50.0);
        l.text(">> This text is floated 50pt down");
    });

    layout.text("After float (continues from 'Before')");
    layout.move_down(60.0); // Make room for floated text

    // Padding
    layout.font("Helvetica").size(14.0);
    layout.text("Padding:");
    layout.move_down(10.0);

    layout.font("Helvetica").size(11.0);
    layout.text("Before padding");

    layout.pad(15.0, |l| {
        l.text("Content with 15pt padding above and below");
    });

    layout.text("After padding");

    layout.move_down(20.0);

    // Side-by-side layout
    layout.font("Helvetica").size(14.0);
    layout.text("Side-by-Side Layout:");
    layout.move_down(10.0);

    let box_top = layout.cursor();

    // Left box
    layout.bounding_box([0.0, 0.0], 150.0, Some(80.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(10.0);
        doc.text("Left Column");
        doc.text("Width: 150pt");
    });

    // Right box (reset cursor to same level)
    layout.set_cursor(box_top);
    layout.bounding_box([170.0, 0.0], 150.0, Some(80.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(10.0);
        doc.text("Right Column");
        doc.text("Offset: 170pt");
    });

    // === Page 5: Summary ===
    layout.start_new_page();

    layout.font("Helvetica").size(20.0);
    layout.text("Summary");
    layout.move_down(15.0);

    layout.font("Helvetica").size(12.0);
    layout.text("This demo showed how LayoutDocument simplifies PDF creation:");
    layout.move_down(10.0);

    layout.indent(20.0, 0.0, |l| {
        l.text("1. No coordinate math - cursor tracks position automatically");
        l.text("2. Headers/footers applied to all (or selected) pages");
        l.text("3. Page numbers with customizable format and position");
        l.text("4. Bounding boxes for isolated layout regions");
        l.text("5. Text alignment and wrapping built-in");
        l.text("6. Helper methods: indent, pad, float");
    });

    layout.move_down(30.0);

    layout.align(TextAlign::Center);
    layout.font("Helvetica").size(14.0);
    layout.text("End of Demo");

    layout.move_down(10.0);
    layout.font("Helvetica").size(10.0);
    layout.text("Generated with pdfcrate LayoutDocument API");

    // Apply headers, footers, and page numbers
    layout.apply_repeaters();

    // Save the document
    layout.save("layout_demo.pdf")?;

    println!("Created: layout_demo.pdf ({} pages)", layout.page_count());

    Ok(())
}
