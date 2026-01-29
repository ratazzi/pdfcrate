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

use pdfcrate::api::AxisOptions;
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
    // Draw coordinate axes for visual reference (gray)
    doc.stroke_axis(
        AxisOptions::new()
            .at(20.0, 20.0)
            .color(0.6, 0.6, 0.6)
            .step_length(100.0),
    );

    // Header band
    doc.fill(|ctx| {
        ctx.color(0.95, 0.95, 0.95)
            .rectangle([0.0, 842.0], 595.0, 82.0);
    });

    doc.font("Helvetica").size(24.0);
    doc.text_at("Drawing Primitives", [48.0, 804.0]);
    doc.font("Helvetica").size(11.0);
    doc.text_at("Strokes, fills, polygons & transparency", [48.0, 784.0]);

    // Section labels
    doc.font("Helvetica").size(12.0);
    doc.text_at("Strokes", [60.0, 720.0]);
    doc.text_at("Fills", [320.0, 720.0]);

    // === Stroke-only shapes ===
    // Blue rectangle
    doc.stroke(|ctx| {
        ctx.color(0.15, 0.45, 0.85)
            .line_width(2.0)
            .rectangle([60.0, 700.0], 180.0, 90.0);
    });
    // Red rounded rectangle
    doc.stroke(|ctx| {
        ctx.color(0.9, 0.3, 0.2).line_width(3.0).rounded_rectangle(
            [60.0, 580.0],
            180.0,
            90.0,
            14.0,
        );
    });
    // Green circle
    doc.stroke(|ctx| {
        ctx.color(0.2, 0.7, 0.4)
            .line_width(2.5)
            .circle([150.0, 420.0], 40.0);
    });
    // Dashed line (matches Ruby's inherited 2.5pt line width)
    doc.stroke(|ctx| {
        ctx.color(0.2, 0.2, 0.2)
            .line_width(2.5)
            .dash(&[6.0, 4.0])
            .line([60.0, 360.0], [240.0, 360.0]);
    });

    // === Filled shapes ===
    // Yellow rounded rectangle
    doc.fill(|ctx| {
        ctx.color(0.98, 0.85, 0.25)
            .rounded_rectangle([320.0, 700.0], 220.0, 90.0, 18.0);
    });
    // Blue ellipse
    doc.fill(|ctx| {
        ctx.color(0.2, 0.62, 0.95)
            .ellipse([430.0, 520.0], 90.0, 45.0);
    });
    // Pink circle
    doc.fill(|ctx| {
        ctx.color(0.9, 0.5, 0.6).circle([430.0, 420.0], 45.0);
    });

    // === Polygons section ===
    doc.font("Helvetica").size(12.0);
    doc.text_at("Polygons", [60.0, 320.0]);

    // Triangle (stroke)
    doc.stroke(|ctx| {
        ctx.color(0.8, 0.2, 0.2).line_width(2.5).polygon(&[
            [100.0, 280.0],
            [140.0, 280.0],
            [120.0, 240.0],
        ]);
    });

    // Pentagon (fill)
    doc.fill(|ctx| {
        ctx.color(0.2, 0.6, 0.8).polygon(&[
            [200.0, 280.0],
            [220.0, 270.0],
            [215.0, 245.0],
            [185.0, 245.0],
            [180.0, 270.0],
        ]);
    });

    // Star (fill)
    doc.fill(|ctx| {
        ctx.color(0.9, 0.8, 0.2).polygon(&[
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

    // Hexagon (stroke + fill with transparency)
    doc.transparent(0.6, 0.6, |doc| {
        doc.fill(|ctx| {
            ctx.color(0.5, 0.3, 0.8).polygon(&[
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
        ctx.color(0.3, 0.1, 0.5).line_width(2.0).polygon(&[
            [430.0, 280.0],
            [450.0, 270.0],
            [450.0, 250.0],
            [430.0, 240.0],
            [410.0, 250.0],
            [410.0, 270.0],
        ]);
    });

    // === Transparency section ===
    doc.font("Helvetica").size(12.0);
    doc.text_at("Transparency", [60.0, 200.0]);

    // Overlapping circles with transparency
    let circle_cx = 120.0;
    let circle_cy = 130.0;

    doc.fill(|ctx| {
        ctx.color(1.0, 0.0, 0.0)
            .circle([circle_cx, circle_cy], 35.0);
    });
    doc.transparent(0.7, 0.7, |d| {
        d.fill(|ctx| {
            ctx.color(0.0, 1.0, 0.0)
                .circle([circle_cx + 40.0, circle_cy], 35.0);
        });
    });
    doc.transparent(0.4, 0.4, |d| {
        d.fill(|ctx| {
            ctx.color(0.0, 0.0, 1.0)
                .circle([circle_cx + 20.0, circle_cy - 35.0], 35.0);
        });
    });

    // Overlapping rectangles with transparency
    let rect_x = 320.0;
    let rect_top_y = 155.0;

    // Red rect (100%)
    doc.fill(|ctx| {
        ctx.color(0.85, 0.2, 0.3)
            .rectangle([rect_x, rect_top_y], 80.0, 55.0);
    });
    // Blue rect (65%)
    doc.transparent(0.65, 0.65, |d| {
        d.fill(|ctx| {
            ctx.color(0.2, 0.6, 0.9)
                .rectangle([rect_x + 45.0, rect_top_y], 80.0, 55.0);
        });
    });
    // Green rect (35%)
    doc.transparent(0.35, 0.35, |d| {
        d.fill(|ctx| {
            ctx.color(0.3, 0.85, 0.3)
                .rectangle([rect_x + 22.0, rect_top_y + 30.0], 80.0, 55.0);
        });
    });

    // Labels
    doc.font("Helvetica").size(9.0);
    doc.text_at("Circles: 100%, 70%, 40%", [60.0, 70.0]);
    doc.text_at("Rectangles: 100%, 65%, 35%", [320.0, 70.0]);

    Ok(())
}
