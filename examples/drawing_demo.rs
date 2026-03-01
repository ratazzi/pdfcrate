//! Drawing Primitives Demo
//!
//! Demonstrates pdfcrate's drawing capabilities:
//! - Coordinate axes for visual reference
//! - Stroke operations (rectangles, rounded rectangles, circles, dashed lines)
//! - Fill operations (rounded rectangles, ellipses, circles)
//! - Polygon drawing (triangles, pentagons, stars, hexagons)
//! - Transparency and alpha blending
//!
//! Run with: cargo run --example drawing_demo

use pdfcrate::api::{AxisOptions, PageLayout, PageSize};
use pdfcrate::prelude::Document;
use pdfcrate::Result as PdfResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("drawing_demo.pdf", |doc| {
        doc.title("Drawing Primitives Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: drawing_demo.pdf");
    Ok(())
}

/// Adds the drawing primitives demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (page_width, page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    doc.stroke_axis(
        AxisOptions::new()
            .at(20.0, 20.0)
            .color("999999")
            .step_length(100.0),
    );

    // Header band
    doc.fill(|ctx| {
        ctx.color("F2F2F2")
            .rectangle([0.0, page_height], page_width, 82.0);
    });

    doc.font("Helvetica").size(24.0);
    doc.text_at("PDF Showcase", [margin, 804.0]);
    doc.font("Helvetica").size(11.0);
    doc.text_at(
        "Drawing primitives, polygons & transparency",
        [margin, 784.0],
    );

    doc.font("Helvetica").size(12.0);
    doc.text_at("Strokes", [60.0, 720.0]);
    doc.text_at("Fills", [320.0, 720.0]);

    // Stroke-only shapes
    doc.stroke(|ctx| {
        ctx.color("2673D9")
            .line_width(2.0)
            .rectangle([60.0, 700.0], 180.0, 90.0);
    });
    doc.stroke(|ctx| {
        ctx.color("E64D33")
            .line_width(3.0)
            .rounded_rectangle([60.0, 580.0], 180.0, 90.0, 14.0);
    });
    doc.stroke(|ctx| {
        ctx.color("33B366")
            .line_width(2.5)
            .circle([150.0, 420.0], 40.0);
    });
    doc.stroke(|ctx| {
        ctx.color("333333")
            .line_width(2.5)
            .dash(&[6.0, 4.0])
            .line([60.0, 360.0], [240.0, 360.0]);
    });

    // Filled shapes
    doc.fill(|ctx| {
        ctx.color("FAD940")
            .rounded_rectangle([320.0, 700.0], 220.0, 90.0, 18.0);
    });
    doc.fill(|ctx| {
        ctx.color("339EF2").ellipse([430.0, 520.0], 90.0, 45.0);
    });
    doc.fill(|ctx| {
        ctx.color("E68099").circle([430.0, 420.0], 45.0);
    });

    // Polygons
    doc.font("Helvetica").size(12.0);
    doc.text_at("Polygons", [60.0, 320.0]);

    doc.stroke(|ctx| {
        ctx.color("CC3333").line_width(2.5).polygon(&[
            [100.0, 280.0],
            [140.0, 280.0],
            [120.0, 240.0],
        ]);
    });

    doc.fill(|ctx| {
        ctx.color("3399CC").polygon(&[
            [200.0, 280.0],
            [220.0, 270.0],
            [215.0, 245.0],
            [185.0, 245.0],
            [180.0, 270.0],
        ]);
    });

    doc.fill(|ctx| {
        ctx.color("E6CC33").polygon(&[
            [310.0, 280.0],
            [315.0, 265.0],
            [330.0, 265.0],
            [320.0, 255.0],
            [325.0, 240.0],
            [310.0, 248.0],
            [295.0, 240.0],
            [300.0, 255.0],
            [290.0, 265.0],
            [305.0, 265.0],
        ]);
    });

    // Hexagon (transparent fill + stroke)
    doc.transparent(0.6, 0.6, |doc| {
        doc.fill(|ctx| {
            ctx.color("804DCC").polygon(&[
                [430.0, 280.0],
                [450.0, 270.0],
                [450.0, 250.0],
                [430.0, 240.0],
                [410.0, 250.0],
                [410.0, 270.0],
            ]);
        });
    });
    doc.stroke(|ctx| {
        ctx.color("4D1A80").line_width(2.0).polygon(&[
            [430.0, 280.0],
            [450.0, 270.0],
            [450.0, 250.0],
            [430.0, 240.0],
            [410.0, 250.0],
            [410.0, 270.0],
        ]);
    });

    // Transparency section
    doc.font("Helvetica").size(12.0);
    doc.text_at("Transparency", [60.0, 200.0]);

    let circle_cx = 120.0;
    let circle_cy = 130.0;

    doc.fill(|ctx| {
        ctx.color("FF0000").circle([circle_cx, circle_cy], 35.0);
    });
    doc.transparent(0.7, 0.7, |d| {
        d.fill(|ctx| {
            ctx.color("00FF00")
                .circle([circle_cx + 40.0, circle_cy], 35.0);
        });
    });
    doc.transparent(0.4, 0.4, |d| {
        d.fill(|ctx| {
            ctx.color("0000FF")
                .circle([circle_cx + 20.0, circle_cy - 35.0], 35.0);
        });
    });

    // Overlapping rectangles
    let rect_x = 320.0;
    let rect_top_y = 155.0;

    doc.fill(|ctx| {
        ctx.color("D93352")
            .rectangle([rect_x, rect_top_y], 80.0, 55.0);
    });
    doc.transparent(0.65, 0.65, |d| {
        d.fill(|ctx| {
            ctx.color("3399E6")
                .rectangle([rect_x + 45.0, rect_top_y], 80.0, 55.0);
        });
    });
    doc.transparent(0.35, 0.35, |d| {
        d.fill(|ctx| {
            ctx.color("4DD94D")
                .rectangle([rect_x + 22.0, rect_top_y + 30.0], 80.0, 55.0);
        });
    });

    // Labels
    doc.font("Helvetica").size(9.0);
    doc.text_at("Circles: 100%, 70%, 40%", [60.0, 70.0]);
    doc.text_at("Rectangles: 100%, 65%, 35%", [320.0, 70.0]);

    Ok(())
}
