//! PDF Embedding Demo
//!
//! Demonstrates pdfcrate's PDF embedding capabilities:
//! - Loading existing PDF documents
//! - Embedding PDF pages as XObjects
//! - Drawing embedded pages as thumbnails
//! - Scaling and positioning embedded content
//!
//! Run with: cargo run --example pdf_embed_demo

use pdfcrate::prelude::{Document, LoadedDocument, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("pdf_embed_demo.pdf", |doc| {
        doc.title("PDF Embed Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: pdf_embed_demo.pdf");
    Ok(())
}

/// Adds the PDF embedding demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    // Header band (top-left: 0, 842)
    doc.fill(|ctx| {
        ctx.gray(0.95).rect_tl([0.0, 842.0], page_width, 82.0);
    });

    doc.font("Helvetica").size(24.0);
    doc.text_at("PDF Embedding", [margin, 800.0]);

    doc.font("Helvetica").size(11.0);
    doc.text_at("Embed and draw pages from other PDFs", [margin, 780.0]);

    // Create a sample "source" PDF in memory
    let source_pdf = create_sample_source_pdf()?;

    // Load the source PDF
    let mut loaded = LoadedDocument::load(source_pdf)?;
    let page_count = loaded.page_count()?;

    doc.font("Helvetica").size(12.0);
    doc.text_at(
        &format!("Source PDF has {} page(s)", page_count),
        [margin, 720.0],
    );

    // Embed all pages from the source
    let embedded_pages = doc.embed_pdf(&mut loaded)?;

    // Draw the embedded pages as thumbnails
    let mut y = 680.0;
    let thumbnail_width = 150.0;
    let thumbnail_height = 200.0;
    let spacing = 20.0;

    doc.font("Helvetica").size(14.0);
    doc.text_at("Embedded Page Thumbnails:", [margin, y]);
    y -= 30.0;

    let mut x = margin;
    for (i, page) in embedded_pages.iter().enumerate() {
        // Draw a border around the thumbnail (using top-left origin)
        doc.stroke(|ctx| {
            ctx.gray(0.7).line_width(1.0).rect_tl(
                [x - 2.0, y + 2.0],
                thumbnail_width + 4.0,
                thumbnail_height + 4.0,
            );
        });

        // Draw the embedded page scaled to fit
        doc.draw_pdf_page_fit(
            page,
            [x, y - thumbnail_height],
            thumbnail_width,
            thumbnail_height,
        );

        // Label
        doc.font("Helvetica").size(10.0);
        doc.text_at(
            &format!(
                "Page {} ({}x{})",
                i + 1,
                page.width as i32,
                page.height as i32
            ),
            [x, y - thumbnail_height - 15.0],
        );

        x += thumbnail_width + spacing;

        // Wrap to next row if needed
        if x + thumbnail_width > page_width - margin {
            x = margin;
            y -= thumbnail_height + 50.0;
        }
    }

    // Add a note about the feature
    doc.font("Helvetica").size(10.0);
    doc.text_at(
        "This demonstrates loading an existing PDF and embedding its pages as XObjects.",
        [margin, 100.0],
    );
    doc.text_at(
        "Use cases: PDF merging, thumbnails, page composition, watermarking.",
        [margin, 85.0],
    );

    Ok(())
}

/// Creates a sample source PDF with multiple pages for embedding demonstration
fn create_sample_source_pdf() -> PdfResult<Vec<u8>> {
    let mut source = Document::new();

    // Page 1: Title page (using top-left coordinates)
    source.fill(|ctx| {
        ctx.color(0.2, 0.4, 0.8).rect_tl([0.0, 842.0], 595.0, 142.0);
    });
    source.font("Helvetica").size(28.0);
    source.text_at("Sample Source PDF", [150.0, 750.0]);
    source.font("Helvetica").size(14.0);
    source.text_at("Page 1 of 3", [250.0, 720.0]);

    source.font("Helvetica").size(12.0);
    source.text_at("This PDF was created in memory", [180.0, 400.0]);
    source.text_at("and embedded into the showcase.", [180.0, 380.0]);

    // Page 2: Shapes
    source.start_new_page();
    source.font("Helvetica").size(18.0);
    source.text_at("Geometric Shapes", [200.0, 780.0]);
    source.font("Helvetica").size(10.0);
    source.text_at("Page 2 of 3", [260.0, 760.0]);

    source.fill(|ctx| {
        ctx.color(0.9, 0.3, 0.3).circle([150.0, 500.0], 80.0);
        // Green square (using top-left coordinates)
        ctx.color(0.3, 0.9, 0.3)
            .rect_tl([280.0, 580.0], 160.0, 160.0);
        ctx.color(0.3, 0.3, 0.9)
            .ellipse([150.0, 300.0], 100.0, 50.0);
    });

    // Page 3: Text content
    source.start_new_page();
    source.font("Helvetica").size(18.0);
    source.text_at("Text Content", [220.0, 780.0]);
    source.font("Helvetica").size(10.0);
    source.text_at("Page 3 of 3", [260.0, 760.0]);

    source.font("Helvetica").size(11.0);
    let lines = [
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
        "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
        "Ut enim ad minim veniam, quis nostrud exercitation ullamco.",
        "Duis aute irure dolor in reprehenderit in voluptate velit.",
        "Excepteur sint occaecat cupidatat non proident.",
    ];

    let mut y = 700.0;
    for line in &lines {
        source.text_at(line, [72.0, y]);
        y -= 20.0;
    }

    source.render()
}
