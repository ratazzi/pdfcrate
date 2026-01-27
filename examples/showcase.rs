//! Showcase PDF demonstrating pdfcrate features:
//! - Drawing primitives (shapes, strokes, fills)
//! - Embedded PNG image
//! - Embedded JPEG image
//! - PNG with alpha transparency
//! - Custom TrueType font embedding (requires `fonts` feature)
//! - MapleMono ligatures (best with `text-shaping` feature)
//! - CJK font support (Chinese/Japanese/Korean)
//! - Interactive forms (AcroForms)
//! - PDF embedding and merging
//! - SVG barcode (path-only, requires `svg` feature)
//! - LayoutDocument - Prawn-style cursor-based layout
//! - Advanced layout: text alignment, leading, wrapping, text boxes
//! - Transparency support for overlapping graphics
//! - Polygon drawing (stroke and fill)
//! - Document outline (bookmarks/table of contents)
//!
//! Run with: cargo run --example showcase --features "fonts,text-shaping,svg"

// Import individual demo modules (each has its own main/helpers unused here)
#[allow(dead_code)]
#[path = "alpha_demo.rs"]
mod alpha_demo;
#[allow(dead_code)]
#[cfg(feature = "fonts")]
#[path = "cjk_demo.rs"]
mod cjk_demo;
#[allow(dead_code)]
#[cfg(feature = "fonts")]
#[path = "custom_font_demo.rs"]
mod custom_font_demo;
#[allow(dead_code)]
#[path = "drawing_demo.rs"]
mod drawing_demo;
#[allow(dead_code)]
#[path = "forms_demo.rs"]
mod forms_demo;
#[allow(dead_code)]
#[path = "grid_demo.rs"]
mod grid_demo;
#[allow(dead_code)]
#[path = "jpeg_demo.rs"]
mod jpeg_demo;
#[allow(dead_code)]
#[path = "layout_advanced_demo.rs"]
mod layout_advanced_demo;
#[allow(dead_code)]
#[path = "layout_demo.rs"]
mod layout_demo;
#[allow(dead_code)]
#[cfg(feature = "fonts")]
#[path = "ligatures_demo.rs"]
mod ligatures_demo;
#[allow(dead_code)]
#[path = "pdf_embed_demo.rs"]
mod pdf_embed_demo;
#[allow(dead_code)]
#[path = "png_demo.rs"]
mod png_demo;
#[allow(dead_code)]
#[cfg(feature = "svg")]
#[path = "svg_barcode_demo.rs"]
mod svg_barcode_demo;
#[allow(dead_code)]
#[path = "text_box_overflow_demo.rs"]
mod text_box_overflow_demo;

use image::codecs::jpeg::JpegEncoder;
use image::ImageReader;
use pdfcrate::image::embed_jpeg;
use pdfcrate::prelude::Document;
use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::result::Result as StdResult;

fn main() -> StdResult<(), Box<dyn Error>> {
    let png_path = "examples/example.png";
    let png_bytes = fs::read(png_path)?;
    let alpha_bytes = png_bytes.clone();

    // Convert PNG to JPEG at runtime for JPEG demo page
    let img = ImageReader::new(Cursor::new(&png_bytes))
        .with_guessed_format()?
        .decode()?;
    let rgb_img = img.to_rgb8();
    let mut jpeg_bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, 85);
    encoder.encode_image(&rgb_img)?;
    let (png_width, png_height) = png_demo::read_png_dimensions(&png_bytes)?;
    let (alpha_width, alpha_height) = png_demo::read_png_dimensions(&alpha_bytes)?;
    let jpeg_info = embed_jpeg(&jpeg_bytes)?;
    let (jpeg_width, jpeg_height) = (jpeg_info.width, jpeg_info.height);

    Document::generate("showcase.pdf", |doc| {
        doc.title("pdfcrate Showcase").author("pdfcrate");

        // Track page indices for outline
        let mut page_idx = 0;
        let drawing_page = page_idx;

        drawing_demo::add_page(doc)?;
        page_idx += 1;
        let png_page = page_idx;

        doc.start_new_page();
        png_demo::add_page(doc, &png_bytes, png_width, png_height)?;
        page_idx += 1;
        let jpeg_page = page_idx;

        doc.start_new_page();
        jpeg_demo::add_page(doc, &jpeg_bytes, jpeg_width, jpeg_height)?;
        page_idx += 1;
        let alpha_page = page_idx;

        doc.start_new_page();
        alpha_demo::add_page(doc, &alpha_bytes, alpha_width, alpha_height)?;
        page_idx += 1;

        #[cfg(feature = "fonts")]
        let custom_font_page = page_idx;
        #[cfg(feature = "fonts")]
        {
            doc.start_new_page();
            custom_font_demo::add_page(doc)?;
            page_idx += 1;
        }

        #[cfg(feature = "fonts")]
        let ligatures_page = page_idx;
        #[cfg(feature = "fonts")]
        {
            doc.start_new_page();
            ligatures_demo::add_page(doc)?;
            page_idx += 1;
        }

        #[cfg(feature = "fonts")]
        let cjk_page = page_idx;
        #[cfg(feature = "fonts")]
        {
            doc.start_new_page();
            cjk_demo::add_page(doc)?;
            page_idx += 1;
        }

        let forms_page = page_idx;
        doc.start_new_page();
        forms_demo::add_page(doc)?;
        page_idx += 1;

        let pdf_embed_page = page_idx;
        doc.start_new_page();
        pdf_embed_demo::add_page(doc)?;
        page_idx += 1;

        #[cfg(feature = "svg")]
        let svg_page = page_idx;
        #[cfg(feature = "svg")]
        {
            doc.start_new_page();
            svg_barcode_demo::add_page(doc)?;
            page_idx += 1;
        }

        let layout_page = page_idx;
        doc.start_new_page();
        layout_demo::add_page(doc)?;
        page_idx += 1;

        let layout_advanced_page = page_idx;
        doc.start_new_page();
        layout_advanced_demo::add_page(doc)?;
        page_idx += 1;

        let text_box_page = page_idx;
        doc.start_new_page();
        text_box_overflow_demo::add_page(doc)?;
        page_idx += 1;

        let grid_page = page_idx;
        doc.start_new_page();
        grid_demo::add_page(doc)?;
        let _ = page_idx; // Suppress unused warning

        // Build document outline (bookmarks)
        doc.outline(|o| {
            // Drawing & Graphics section
            o.section("Drawing & Graphics", drawing_page, |o| {
                o.page("Primitives & Transparency", drawing_page);
                o.page("Polygons", drawing_page);
            });

            // Images section
            o.section("Images", png_page, |o| {
                o.page("PNG Image", png_page);
                o.page("JPEG Image", jpeg_page);
                o.page("PNG with Alpha", alpha_page);
            });

            // Fonts section (only if fonts feature is enabled)
            #[cfg(feature = "fonts")]
            o.section("Fonts", custom_font_page, |o| {
                o.page("Custom TrueType Font", custom_font_page);
                o.page("Ligatures & Kerning", ligatures_page);
                o.page("CJK Support", cjk_page);
            });

            // Interactive section
            o.section("Interactive", forms_page, |o| {
                o.page("AcroForms", forms_page);
                o.page("PDF Embedding", pdf_embed_page);
                #[cfg(feature = "svg")]
                o.page("SVG Barcode", svg_page);
            });

            // Layout section
            o.section("Layout System", layout_page, |o| {
                o.page("LayoutDocument Basics", layout_page);
                o.page("Text Layout Features", layout_advanced_page);
                o.page("Text Box Overflow", text_box_page);
                o.page("Grid System", grid_page);
            });
        });

        Ok(())
    })?;

    println!("Created: showcase.pdf");
    Ok(())
}
