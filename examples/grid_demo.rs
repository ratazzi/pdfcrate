//! Grid System Demo
//!
//! Demonstrates pdfcrate's Prawn-style grid layout system:
//! - Define rows, columns, and gutters
//! - Place content in individual grid cells
//! - Span multiple rows and columns
//! - Create complex layouts with ease
//!
//! Run with: cargo run --example grid_demo

use pdfcrate::api::GridOptions;
use pdfcrate::prelude::{Document, LayoutDocument, Margin};
use pdfcrate::Result as PdfResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("grid_demo.pdf", |doc| {
        doc.title("Grid Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: grid_demo.pdf");
    Ok(())
}

/// Adds the grid system demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let mut layout =
        LayoutDocument::with_margin(std::mem::take(doc), Margin::new(36.0, 36.0, 36.0, 36.0));

    // Define a 6-row, 4-column grid with 10pt gutters
    layout.define_grid(GridOptions::new(6, 4).gutter(10.0));

    // Row 0: Header cells with title
    layout.grid_span_bounding_box((0, 0), (0, 3), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica-Bold").size(24.0);
        doc.text("Grid System");
        doc.move_down(5.0);
        doc.font("Helvetica").size(11.0);
        doc.text("Grid layout for precise positioning");
    });

    // Row 1: Vertical span and large span
    layout.grid_span_bounding_box((1, 0), (2, 0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica-Bold").size(10.0);
        doc.text("Vertical");
        doc.font("Helvetica").size(9.0);
        doc.text("(1,0)-(2,0)");
    });

    layout.grid_span_bounding_box((1, 1), (2, 3), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica-Bold").size(12.0);
        doc.text("Large Span (1,1)-(2,3)");
        doc.move_down(5.0);
        doc.font("Helvetica").size(10.0);
        doc.text("This span covers 2 rows and 3 columns.");
        doc.text("Perfect for content areas, sidebars, or");
        doc.text("any layout requiring multiple cells.");
    });

    // Row 3-4: Nav, Main Content, Side
    layout.grid_bounding_box(3, 0, |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(9.0);
        doc.text("Nav");
    });

    layout.grid_span_bounding_box((3, 1), (4, 3), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica-Bold").size(11.0);
        doc.text("Main Content Area");
        doc.move_down(5.0);
        doc.font("Helvetica").size(10.0);
        doc.text("Grids make it easy to create responsive layouts.");
        doc.text("Define rows, columns, and gutters, then place");
        doc.text("content in cells or spans as needed.");
    });

    layout.grid_bounding_box(4, 0, |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(9.0);
        doc.text("Side");
    });

    // Row 5: Footer cells
    for col in 0..4 {
        layout.grid_bounding_box(5, col, |doc| {
            doc.stroke_bounds();
            doc.font("Helvetica").size(9.0);
            doc.text(&format!("Footer {}", col + 1));
        });
    }

    *doc = layout.into_inner();
    Ok(())
}
