//! Ligatures and Kerning Demo
//!
//! Demonstrates pdfcrate's advanced text rendering:
//! - Programming ligatures (MapleMono)
//! - Nerd Font icon glyphs
//! - Kerning comparison (on/off)
//! - Line spacing control
//!
//! Run with: cargo run --example ligatures_demo --features fonts

use pdfcrate::prelude::{Document, PageLayout, PageSize};
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

    let font_name = doc.embed_font(fs::read(MAPLE_FONT_PATH)?)?;

    doc.font(&font_name).size(28.0);
    doc.text_at("MapleMono Ligatures", [margin, 800.0]);

    doc.font("Helvetica").size(12.0);
    doc.text_at("Ligatures, kerning, and line spacing", [margin, 778.0]);

    let mut y = 700.0;
    doc.font(&font_name).size(22.0);

    let samples = ["== != === !== <= >= -> => <-> <=>"];

    for line in samples {
        doc.text_at(line, [margin, y]);
        y -= 32.0;
    }

    y -= 8.0;
    doc.font("Helvetica").size(11.0);
    doc.text_at("Nerd Font glyphs (MapleMono NF):", [margin, y]);
    y -= 32.0;

    doc.font(&font_name).size(24.0);
    doc.text_at(
        "\u{f09b}  \u{f121}  \u{f179}  \u{f0f3}  \u{f0e0}  \u{f2db}  \u{f1eb}",
        [margin, y],
    );
    y -= 36.0;

    doc.stroke(|ctx| {
        ctx.gray(0.88)
            .line_width(0.5)
            .line([margin, y], [page_width - margin, y]);
    });
    y -= 16.0;

    doc.font("Helvetica").size(12.0);
    doc.text_at("Kerning samples (Roboto, proportional):", [margin, y]);
    y -= 18.0;

    let roboto_font = doc.embed_font(fs::read(ROBOTO_FONT_PATH)?)?;
    doc.font("Helvetica").size(10.0);
    doc.text_at("Kerning OFF:", [margin, y]);
    y -= 26.0;

    doc.font(&roboto_font).size(32.0);
    doc.text_at_no_kerning("AV AVA WA We To Ta Te Yo", [margin, y]);
    y -= 48.0;

    doc.font("Helvetica").size(10.0);
    doc.text_at("Kerning ON:", [margin, y]);
    y -= 26.0;

    doc.font(&roboto_font).size(32.0);
    doc.text_at("AV AVA WA We To Ta Te Yo", [margin, y]);
    y -= 48.0;

    doc.font("Helvetica").size(12.0);
    doc.text_at("Line spacing (manual):", [margin, y]);
    y -= 18.0;

    for spacing in [16.0, 24.0, 36.0] {
        doc.font("Helvetica").size(10.0);
        doc.text_at(&format!("Line height {:.0}pt", spacing), [margin, y]);
        let text_y = y - 14.0;

        doc.stroke(|ctx| {
            ctx.gray(0.8)
                .line_width(0.5)
                .line([margin, text_y], [page_width - margin, text_y])
                .line(
                    [margin, text_y - spacing],
                    [page_width - margin, text_y - spacing],
                );
        });

        doc.font(&font_name).size(14.0);
        doc.text_at("The quick brown fox jumps.", [margin, text_y]);
        doc.text_at("Second line for spacing.", [margin, text_y - spacing]);

        y = text_y - spacing - 24.0;
    }

    Ok(())
}
