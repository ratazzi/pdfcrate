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
        ctx.gray(0.95).rect_tl([0.0, 842.0], page_width, 82.0);
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

    // Create LayoutDocument for cursor-based layout
    // Use left margin of 48pt to align with header (which is at x=48)
    // Top margin: header is 82pt, plus ~54pt gap = 136pt from page top
    let doc_owned = std::mem::take(doc);
    let mut layout = LayoutDocument::with_margin(doc_owned, Margin::new(136.0, 48.0, 48.0, 48.0));

    let box_width = 220.0;
    let box_height = 50.0;
    let padding = 4.0;

    // Section 1: Overflow::Truncate
    layout.font("Helvetica-Bold").size(14.0);
    layout.text("1. Overflow::Truncate (default)");
    layout.move_down(8.0);

    layout.font("Helvetica").size(9.0);
    layout.text("Text that exceeds the box height is silently discarded:");
    layout.move_down(10.0);

    // Draw border and text in the same bounding_box
    let mut truncate_result = None;
    layout.bounding_box(
        [0.0, 0.0],
        box_width + padding * 2.0,
        Some(box_height + padding * 2.0),
        |l| {
            l.stroke_bounds();
            l.font("Helvetica").size(9.0);
            truncate_result = Some(l.text_box(
                long_text,
                [padding, padding],
                box_width,
                box_height,
                Overflow::Truncate,
            ));
        },
    );
    let result = truncate_result.unwrap();

    layout.move_down(10.0);
    layout.font("Helvetica").size(8.0);
    layout.text(&format!(
        "Result: truncated={}, lines_rendered={}, total_lines={}",
        result.truncated, result.lines_rendered, result.total_lines
    ));

    layout.move_down(25.0);

    // Section 2: Overflow::ShrinkToFit
    layout.font("Helvetica-Bold").size(14.0);
    layout.text("2. Overflow::ShrinkToFit(min_size)");
    layout.move_down(8.0);

    layout.font("Helvetica").size(9.0);
    layout.text("Font size is reduced until text fits (minimum 6pt):");
    layout.move_down(10.0);

    let mut shrink_result = None;
    layout.bounding_box(
        [0.0, 0.0],
        box_width + padding * 2.0,
        Some(box_height + padding * 2.0),
        |l| {
            l.stroke_bounds();
            l.font("Helvetica").size(9.0);
            shrink_result = Some(l.text_box(
                long_text,
                [padding, padding],
                box_width,
                box_height,
                Overflow::ShrinkToFit(6.0),
            ));
        },
    );
    let result = shrink_result.unwrap();

    layout.move_down(10.0);
    layout.font("Helvetica").size(8.0);
    layout.text(&format!(
        "Result: font_size={:.1}pt (was 9pt), truncated={}, lines={}",
        result.font_size, result.truncated, result.lines_rendered
    ));

    layout.move_down(25.0);

    // Section 3: Overflow::Expand
    layout.font("Helvetica-Bold").size(14.0);
    layout.text("3. Overflow::Expand");
    layout.move_down(8.0);

    layout.font("Helvetica").size(9.0);
    layout.text("Box height expands to fit all content:");
    layout.move_down(10.0);

    // For Expand, render text first, then draw border with actual height
    let cursor_before = layout.cursor();
    layout.font("Helvetica").size(9.0);
    let result = layout.text_box(
        long_text,
        [padding, padding],
        box_width,
        box_height,
        Overflow::Expand,
    );

    // Draw border around the expanded content using float
    layout.float(|l| {
        l.set_cursor(cursor_before);
        l.bounding_box(
            [0.0, 0.0],
            box_width + padding * 2.0,
            Some(result.height + padding * 2.0),
            |l| {
                l.stroke_bounds();
            },
        );
    });

    layout.move_down(10.0);
    layout.font("Helvetica").size(8.0);
    layout.text(&format!(
        "Result: actual_height={:.1}pt (min {}pt), lines={}",
        result.height, box_height, result.lines_rendered
    ));

    layout.move_down(25.0);

    // Section 4: Comparison - same text, all three modes side by side
    layout.font("Helvetica-Bold").size(14.0);
    layout.text("4. Side-by-Side Comparison");
    layout.move_down(8.0);

    layout.font("Helvetica").size(9.0);
    layout.text("Same text in 150x45pt boxes:");
    layout.move_down(10.0);

    let compare_width = 150.0;
    let compare_height = 45.0; // Smaller height to show overflow effects
    let gap = 15.0;
    let small_padding = 2.0;
    // Longer text to clearly show overflow differences
    let compare_text =
        "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. How vexingly quick daft zebras jump!";

    let row_top = layout.cursor();

    // Box 1: Truncate
    layout.bounding_box([0.0, 0.0], compare_width, Some(compare_height), |l| {
        l.stroke_bounds();
        l.font("Helvetica").size(9.0);
        l.text_box(
            compare_text,
            [small_padding, small_padding],
            compare_width - small_padding * 2.0,
            compare_height - small_padding * 2.0,
            Overflow::Truncate,
        );
    });

    // Box 2: ShrinkToFit
    layout.set_cursor(row_top);
    layout.bounding_box(
        [compare_width + gap, 0.0],
        compare_width,
        Some(compare_height),
        |l| {
            l.stroke_bounds();
            l.font("Helvetica").size(9.0);
            l.text_box(
                compare_text,
                [small_padding, small_padding],
                compare_width - small_padding * 2.0,
                compare_height - small_padding * 2.0,
                Overflow::ShrinkToFit(5.0),
            );
        },
    );

    // Box 3: Expand - render text first, then draw border
    layout.set_cursor(row_top);
    layout.font("Helvetica").size(9.0);
    let result = layout.text_box(
        compare_text,
        [(compare_width + gap) * 2.0 + small_padding, small_padding],
        compare_width - small_padding * 2.0,
        compare_height - small_padding * 2.0,
        Overflow::Expand,
    );

    // Draw border for expanded box
    layout.float(|l| {
        l.set_cursor(row_top);
        l.bounding_box(
            [(compare_width + gap) * 2.0, 0.0],
            compare_width,
            Some(result.height + small_padding * 2.0),
            |l| {
                l.stroke_bounds();
            },
        );
    });

    // Labels below the boxes - find the tallest box height
    let max_box_height = compare_height.max(result.height + small_padding * 2.0);
    layout.set_cursor(row_top - max_box_height - 5.0);

    // Use absolute coordinates for labels (add left margin of 48.0)
    let left_margin = 48.0;
    let label_y = layout.cursor();
    layout.font("Helvetica").size(7.0);
    layout
        .inner_mut()
        .text_at("Truncate", [left_margin + small_padding, label_y]);
    layout.inner_mut().text_at(
        "ShrinkToFit(5.0)",
        [left_margin + compare_width + gap + small_padding, label_y],
    );
    layout.inner_mut().text_at(
        &format!("Expand (h={:.0})", result.height + small_padding * 2.0),
        [
            left_margin + (compare_width + gap) * 2.0 + small_padding,
            label_y,
        ],
    );

    *doc = layout.into_inner();
    Ok(())
}
