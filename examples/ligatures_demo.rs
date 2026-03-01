//! Ligatures and Kerning Demo
//!
//! Demonstrates pdfcrate's advanced text rendering:
//! - Programming ligatures (MapleMono)
//! - Nerd Font icon glyphs
//! - Kerning comparison (on/off)
//! - Line spacing control
//!
//! Run with: cargo run --example ligatures_demo --features fonts

use pdfcrate::prelude::{Document, LayoutDocument, Margin, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;
use std::fs;

// Run `./examples/download-fonts.sh` first
const MAPLE_FONT_PATH: &str = "examples/fonts/MapleMono-NF-CN-Regular.ttf";
const ROBOTO_FONT_PATH: &str = "examples/fonts/Roboto-Regular.ttf";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("ligatures_demo.pdf", |doc| {
        doc.title("Ligatures Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: ligatures_demo.pdf");
    Ok(())
}

/// Adds the ligatures and kerning demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    // Header band (top-left: 0, 842)
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 842.0], page_width, 82.0);
    });

    let maple_mono = doc.embed_font(fs::read(MAPLE_FONT_PATH)?)?;

    doc.font(&maple_mono).size(28.0);
    doc.text_at("MapleMono Ligatures", [margin, 800.0]);

    doc.font("Helvetica").size(12.0);
    doc.text_at("Ligatures, kerning, and line spacing", [margin, 778.0]);

    // Wrap into LayoutDocument for cursor-based body
    // Prawn: margin 36, indent(12) → text at 48
    let doc_owned = std::mem::take(doc);
    let mut layout = LayoutDocument::with_margin(
        doc_owned,
        Margin::new(142.0, margin - 12.0, margin - 12.0, margin - 12.0),
    );

    let roboto_font = layout.embed_font(fs::read(ROBOTO_FONT_PATH)?)?;

    let abs_bottom = layout.bounds().absolute_bottom();

    layout.indent(12.0, 0.0, |layout| {
        // Prawn drawing coords are relative to indented bounds
        let bounds_left = layout.bounds().absolute_left();
        // Ligature samples
        layout.font(&maple_mono).size(22.0);
        layout.text("== != === !== <= >= -> => <-> <=>");
        layout.move_down(8.0);

        layout.font("Helvetica").size(11.0);
        layout.text("Nerd Font glyphs (MapleMono NF):");
        layout.move_down(16.0);

        layout.font(&maple_mono).size(24.0);
        layout.text("\u{f09b}  \u{f121}  \u{f179}  \u{f0f3}  \u{f0e0}  \u{f2db}  \u{f1eb}");
        layout.move_down(26.0);

        layout.font("Helvetica").size(12.0);
        layout.text("Kerning samples (Roboto, proportional):");

        // Kerning OFF
        layout.font("Helvetica").size(10.0);
        layout.text("Kerning OFF:");
        layout.move_down(26.0);

        let y = layout.cursor() + abs_bottom;
        layout.font(&roboto_font).size(32.0);
        layout.text_at_no_kerning("AV AVA WA We To Ta Te Yo", [bounds_left, y]);
        layout.move_down(22.0);

        // Kerning ON
        layout.font("Helvetica").size(10.0);
        layout.text("Kerning ON:");
        layout.move_down(26.0);

        let y = layout.cursor() + abs_bottom;
        layout.font(&roboto_font).size(32.0);
        layout.text_at("AV AVA WA We To Ta Te Yo", [bounds_left, y]);
        layout.move_down(22.0);

        // Line spacing
        layout.font("Helvetica").size(12.0);
        layout.text("Line spacing (manual):");

        let right_x = page_width - margin * 2.0;
        for spacing in [16.0, 24.0, 36.0] {
            layout.font("Helvetica").size(10.0);
            layout.text(&format!("Line height {:.0}pt", spacing));
            layout.move_down(12.0);

            let text_y = layout.cursor() + abs_bottom;

            layout.stroke(|ctx| {
                ctx.color("999999")
                    .line_width(0.5)
                    .line([bounds_left, text_y], [bounds_left + right_x, text_y])
                    .line(
                        [bounds_left, text_y - spacing],
                        [bounds_left + right_x, text_y - spacing],
                    );
            });

            layout.font(&maple_mono).size(14.0);
            layout.text_at("The quick brown fox jumps.", [bounds_left, text_y]);
            layout.text_at("Second line for spacing.", [bounds_left, text_y - spacing]);

            layout.move_cursor_to(text_y - abs_bottom - spacing - 24.0);
        }
    });

    *doc = layout.into_inner();
    Ok(())
}
