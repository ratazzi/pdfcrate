//! SVG Barcode Demo
//!
//! Demonstrates pdfcrate's SVG rendering capabilities:
//! - Rendering SVG content in PDF
//! - Scaling SVG to fit target dimensions
//! - Using SVG for vector graphics like barcodes
//!
//! Run with: cargo run --example svg_barcode_demo --features svg

use pdfcrate::prelude::{Document, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("svg_barcode_demo.pdf", |doc| {
        doc.title("SVG Barcode Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: svg_barcode_demo.pdf");
    Ok(())
}

/// Adds the SVG barcode demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    // Header band (top-left: 0, 842)
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 842.0], page_width, 82.0);
    });

    doc.font("Helvetica").size(24.0);
    doc.text_at("SVG Barcode", [margin, 800.0]);

    doc.font("Helvetica").size(11.0);
    doc.text_at("SVG rendering", [margin, 780.0]);

    doc.font("Helvetica").size(12.0);
    doc.text_at(
        "The barcode below is drawn from SVG (basic shapes are converted to paths).",
        [margin, 720.0],
    );

    let barcode_svg = r##"
<svg xmlns="http://www.w3.org/2000/svg" width="220" height="80" viewBox="0 0 220 80">
  <rect x="8" y="8" width="4" height="64" fill="#000"/>
  <rect x="16" y="8" width="2" height="64" fill="#000"/>
  <rect x="22" y="8" width="6" height="64" fill="#000"/>
  <rect x="32" y="8" width="2" height="64" fill="#000"/>
  <rect x="38" y="8" width="4" height="64" fill="#000"/>
  <rect x="46" y="8" width="2" height="64" fill="#000"/>
  <rect x="52" y="8" width="6" height="64" fill="#000"/>
  <rect x="62" y="8" width="2" height="64" fill="#000"/>
  <rect x="68" y="8" width="4" height="64" fill="#000"/>
  <rect x="76" y="8" width="2" height="64" fill="#000"/>
  <rect x="82" y="8" width="6" height="64" fill="#000"/>
  <rect x="92" y="8" width="2" height="64" fill="#000"/>
  <rect x="98" y="8" width="4" height="64" fill="#000"/>
  <rect x="106" y="8" width="2" height="64" fill="#000"/>
  <rect x="112" y="8" width="6" height="64" fill="#000"/>
  <rect x="122" y="8" width="2" height="64" fill="#000"/>
  <rect x="128" y="8" width="4" height="64" fill="#000"/>
  <rect x="136" y="8" width="2" height="64" fill="#000"/>
  <rect x="142" y="8" width="6" height="64" fill="#000"/>
  <rect x="152" y="8" width="2" height="64" fill="#000"/>
  <rect x="158" y="8" width="4" height="64" fill="#000"/>
  <rect x="166" y="8" width="2" height="64" fill="#000"/>
  <rect x="172" y="8" width="6" height="64" fill="#000"/>
  <rect x="182" y="8" width="2" height="64" fill="#000"/>
  <rect x="188" y="8" width="4" height="64" fill="#000"/>
  <rect x="196" y="8" width="2" height="64" fill="#000"/>
  <rect x="202" y="8" width="6" height="64" fill="#000"/>
</svg>
"##;

    let target_width = page_width - margin * 2.0;
    let target_height = 140.0;
    let x = margin;
    let y = 520.0;

    // Background rectangles (using top-left origin)
    let rect_top_y = y + target_height + 8.0;
    doc.fill(|ctx| {
        ctx.gray(0.97).rectangle(
            [x - 8.0, rect_top_y],
            target_width + 16.0,
            target_height + 16.0,
        );
    });
    doc.stroke(|ctx| {
        ctx.gray(0.85).line_width(0.5).rectangle(
            [x - 8.0, rect_top_y],
            target_width + 16.0,
            target_height + 16.0,
        );
    });

    doc.draw_svg(barcode_svg, [x, y], target_width, target_height)?;

    doc.font("Helvetica").size(10.0);
    doc.text_at(
        "Use SVG for barcodes, charts, and icons without rasterization.",
        [margin, 470.0],
    );

    Ok(())
}
