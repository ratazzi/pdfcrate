//! Showcase PDF demonstrating pdfcrate features:
//! - Drawing primitives (shapes, strokes, fills)
//! - Embedded PNG image
//! - Custom TrueType font embedding (requires `fonts` feature)
//! - MapleMono ligatures (best with `text-shaping` feature)
//! - CJK font support (Chinese/Japanese/Korean)
//! - Interactive forms (AcroForms)
//! - LayoutDocument - cursor-based layout
//! - Advanced layout: text alignment, leading, wrapping, text boxes
//! - Text box overflow modes
//! - Grid layout system
//! - Document outline (bookmarks/table of contents)
//!
//! Run with: cargo run --example showcase --features "fonts,text-shaping"

// Import individual demo modules (each has its own main/helpers unused here)
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
#[path = "png_demo.rs"]
mod png_demo;
#[allow(dead_code)]
#[path = "text_box_overflow_demo.rs"]
mod text_box_overflow_demo;

use pdfcrate::prelude::Document;
use std::error::Error;
use std::fs;
use std::result::Result as StdResult;

fn main() -> StdResult<(), Box<dyn Error>> {
    let png_path = "examples/example.png";
    let png_bytes = fs::read(png_path)?;
    let (png_width, png_height) = png_demo::read_png_dimensions(&png_bytes)?;

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
            o.page("PNG Image", png_page);

            // Fonts section (only if fonts feature is enabled)
            #[cfg(feature = "fonts")]
            o.section("Fonts", custom_font_page, |o| {
                o.page("Custom TrueType Font", custom_font_page);
                o.page("Ligatures & Kerning", ligatures_page);
                o.page("CJK Support", cjk_page);
            });

            // Forms
            o.page("Forms", forms_page);

            // Layout section
            o.section("Layout System", layout_page, |o| {
                o.page("Cursor Layout", layout_page);
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
