//! Text Box Overflow Demo
//!
//! Demonstrates pdfcrate's text_box overflow modes:
//! - Truncate: Silently discard text that exceeds the box height
//! - ShrinkToFit: Reduce font size until text fits
//! - Expand: Expand box height to fit all content
//!
//! Run with: cargo run --example text_box_overflow_demo

use pdfcrate::prelude::{Document, LayoutDocument, Margin, Overflow, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("text_box_overflow_demo.pdf", |doc| {
        doc.title("Text Box Overflow Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: text_box_overflow_demo.pdf");
    Ok(())
}

/// Adds the text box overflow modes demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);

    // Header (top-left: 0, 842)
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 842.0], page_width, 82.0);
    });
    doc.font("Helvetica").size(24.0);
    doc.text_at("Text Box Overflow Modes", [48.0, 804.0]);
    doc.font("Helvetica").size(11.0);
    doc.text_at("Truncate, ShrinkToFit, and Expand behaviors", [48.0, 784.0]);

    // Sample text that will overflow
    let long_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
        Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
        Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. \
        Duis aute irure dolor in reprehenderit in voluptate velit esse cillum.";

    // Match Prawn: default margins (36pt) + move_cursor_to bounds.top - 100
    let doc_owned = std::mem::take(doc);
    let mut layout = LayoutDocument::with_margin(doc_owned, Margin::new(36.0, 36.0, 36.0, 36.0));

    let bounds_top = layout.bounds().height();
    layout.move_cursor_to(bounds_top - 100.0);

    let box_width = 220.0;
    let box_height = 50.0;
    let padding = 4.0;
    let left_offset = 12.0;

    // Section 1: Overflow::Truncate
    layout.indent(left_offset, 0.0, |l| {
        l.font("Helvetica-Bold").size(14.0);
        l.text("1. Overflow: :truncate (default)");
        l.move_down(8.0);

        l.font("Helvetica").size(9.0);
        l.text("Text that exceeds the box height is silently discarded:");
    });
    layout.move_down(10.0);

    let y = layout.cursor();
    let outer_height = box_height + padding * 2.0;
    layout.bounding_box(
        [left_offset, y],
        box_width + padding * 2.0,
        Some(outer_height),
        |l| {
            l.stroke_bounds();
            l.font("Helvetica").size(9.0);
            l.text_box(
                long_text,
                [padding, box_height + padding],
                box_width,
                box_height,
                Overflow::Truncate,
            );
        },
    );

    layout.move_down(10.0);
    layout.indent(left_offset, 0.0, |l| {
        l.font("Helvetica").size(8.0);
        l.text("Result: truncated (overflow: :truncate)");
    });

    layout.move_down(25.0);

    // Section 2: Overflow::ShrinkToFit
    layout.indent(left_offset, 0.0, |l| {
        l.font("Helvetica-Bold").size(14.0);
        l.text("2. Overflow: :shrink_to_fit");
        l.move_down(8.0);

        l.font("Helvetica").size(9.0);
        l.text("Font size is reduced until text fits (minimum 6pt):");
    });
    layout.move_down(10.0);

    let y = layout.cursor();
    let outer_height = box_height + padding * 2.0;
    layout.bounding_box(
        [left_offset, y],
        box_width + padding * 2.0,
        Some(outer_height),
        |l| {
            l.stroke_bounds();
            l.font("Helvetica").size(9.0);
            l.text_box(
                long_text,
                [padding, box_height + padding],
                box_width,
                box_height,
                Overflow::ShrinkToFit(6.0),
            );
        },
    );

    layout.move_down(10.0);
    layout.indent(left_offset, 0.0, |l| {
        l.font("Helvetica").size(8.0);
        l.text("Result: font shrunk to fit (overflow: :shrink_to_fit, min_font_size: 6)");
    });

    layout.move_down(25.0);

    // Section 3: Overflow::Expand
    layout.indent(left_offset, 0.0, |l| {
        l.font("Helvetica-Bold").size(14.0);
        l.text("3. Overflow: :expand");
        l.move_down(8.0);

        l.font("Helvetica").size(9.0);
        l.text("Box height expands to fit all content:");
    });
    layout.move_down(10.0);

    let cursor_before = layout.cursor();
    layout.font("Helvetica").size(9.0);
    let _result = layout.text_box(
        long_text,
        [left_offset + padding, cursor_before - padding],
        box_width,
        box_height,
        Overflow::Expand,
    );

    // Match Ruby: fixed-height stroke_rectangle
    layout.float(|l| {
        l.set_cursor(cursor_before);
        l.bounding_box(
            [left_offset, cursor_before],
            box_width + padding * 2.0,
            Some(box_height + padding * 2.0 + 20.0),
            |l| {
                l.stroke_bounds();
            },
        );
    });

    // Match Ruby: move_cursor_to box_top - box_height - padding * 2 - 30
    layout.move_cursor_to(cursor_before - box_height - padding * 2.0 - 30.0);

    layout.indent(left_offset, 0.0, |l| {
        l.font("Helvetica").size(8.0);
        l.text("Result: box expanded (overflow: :expand)");
    });

    layout.move_down(25.0);

    // Section 4: Comparison - same text, all three modes side by side
    layout.indent(left_offset, 0.0, |l| {
        l.font("Helvetica-Bold").size(14.0);
        l.text("4. Side-by-Side Comparison");
        l.move_down(8.0);

        l.font("Helvetica").size(9.0);
        l.text("Same text in 150x45pt boxes:");
    });
    layout.move_down(10.0);

    let compare_width = 150.0;
    let compare_height = 45.0;
    let gap = 15.0;
    let small_padding = 2.0;
    let compare_text =
        "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. How vexingly quick daft zebras jump!";

    let row_top = layout.cursor();

    // Box 1: Truncate
    layout.bounding_box(
        [left_offset, row_top],
        compare_width,
        Some(compare_height),
        |l| {
            l.stroke_bounds();
            l.font("Helvetica").size(9.0);
            l.text_box(
                compare_text,
                [small_padding, compare_height - small_padding],
                compare_width - small_padding * 2.0,
                compare_height - small_padding * 2.0,
                Overflow::Truncate,
            );
        },
    );

    // Box 2: ShrinkToFit
    layout.set_cursor(row_top);
    layout.bounding_box(
        [left_offset + compare_width + gap, row_top],
        compare_width,
        Some(compare_height),
        |l| {
            l.stroke_bounds();
            l.font("Helvetica").size(9.0);
            l.text_box(
                compare_text,
                [small_padding, compare_height - small_padding],
                compare_width - small_padding * 2.0,
                compare_height - small_padding * 2.0,
                Overflow::ShrinkToFit(5.0),
            );
        },
    );

    // Box 3: Expand - render text first, then draw border
    let expand_x = left_offset + (compare_width + gap) * 2.0;
    layout.set_cursor(row_top);
    layout.font("Helvetica").size(9.0);
    let _result = layout.text_box(
        compare_text,
        [expand_x + small_padding, row_top - small_padding],
        compare_width - small_padding * 2.0,
        compare_height - small_padding * 2.0,
        Overflow::Expand,
    );

    // Match Ruby: fixed-height stroke_rectangle for expand box
    layout.float(|l| {
        l.set_cursor(row_top);
        l.bounding_box(
            [expand_x, row_top],
            compare_width,
            Some(compare_height + 10.0),
            |l| {
                l.stroke_bounds();
            },
        );
    });

    // Match Ruby: move_cursor_to row_top - compare_height - 15
    layout.move_cursor_to(row_top - compare_height - 15.0);

    // Labels below boxes using absolute coordinates
    let abs_left = 36.0; // page left margin
    let label_y = layout.bounds().absolute_bottom() + layout.cursor();
    layout.font("Helvetica").size(7.0);
    layout.inner_mut().text_at(
        "Truncate",
        [abs_left + left_offset + small_padding, label_y],
    );
    layout.inner_mut().text_at(
        "ShrinkToFit(5.0)",
        [
            abs_left + left_offset + compare_width + gap + small_padding,
            label_y,
        ],
    );
    layout
        .inner_mut()
        .text_at("Expand", [abs_left + expand_x + small_padding, label_y]);

    *doc = layout.into_inner();
    Ok(())
}
