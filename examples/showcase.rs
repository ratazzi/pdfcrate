//! Showcase PDF with drawing on page 1 and an embedded image on page 2.
//!
//! Run with: cargo run --example showcase

use pdf_rs::image::embed_jpeg;
use pdf_rs::prelude::{Document, PageLayout, PageSize};
use pdf_rs::Result as PdfResult;
use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::result::Result as StdResult;

fn main() -> StdResult<(), Box<dyn Error>> {
    let png_path = "example.png";
    let jpeg_path = "example.jpg";
    let alpha_path = "example-alpha.png";
    let png_bytes = fs::read(png_path)?;
    let jpeg_bytes = fs::read(jpeg_path)?;
    let alpha_bytes = fs::read(alpha_path)?;
    let (png_width, png_height) = read_png_dimensions(&png_bytes)?;
    let (alpha_width, alpha_height) = read_png_dimensions(&alpha_bytes)?;
    let jpeg_info = embed_jpeg(&jpeg_bytes)?;
    let (jpeg_width, jpeg_height) = (jpeg_info.width, jpeg_info.height);

    Document::generate("showcase.pdf", |doc| {
        doc.title("pdf_rs Showcase").author("pdf_rs");

        add_page_drawing(doc)?;
        add_page_png(doc, &png_bytes, png_width, png_height)?;
        add_page_jpeg(doc, &jpeg_bytes, jpeg_width, jpeg_height)?;
        add_page_alpha(doc, &alpha_bytes, alpha_width, alpha_height)?;

        Ok(())
    })?;

    println!("Created: showcase.pdf");
    Ok(())
}

fn add_page_drawing(doc: &mut Document) -> PdfResult<()> {
    // Header band
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 760.0], 595.0, 82.0);
    });

    doc.font("Helvetica").size(24.0);
    doc.text_at("pdf_rs Showcase", [48.0, 804.0]);
    doc.font("Helvetica").size(11.0);
    doc.text_at("Page 1: drawing primitives", [48.0, 784.0]);

    doc.font("Helvetica").size(12.0);
    doc.text_at("Strokes", [60.0, 720.0]);
    doc.text_at("Fills", [320.0, 720.0]);

    // Stroke-only shapes
    doc.stroke(|ctx| {
        ctx.color(0.15, 0.45, 0.85)
            .line_width(2.0)
            .rectangle([60.0, 610.0], 180.0, 90.0);
        ctx.color(0.9, 0.3, 0.2)
            .line_width(3.0)
            .rounded_rectangle([60.0, 490.0], 180.0, 90.0, 14.0);
        ctx.color(0.2, 0.7, 0.4)
            .line_width(2.5)
            .circle([150.0, 420.0], 40.0);
        ctx.color(0.2, 0.2, 0.2)
            .dash(&[6.0, 4.0])
            .line([60.0, 360.0], [240.0, 360.0])
            .undash();
    });

    // Filled shapes
    doc.fill(|ctx| {
        ctx.color(0.98, 0.85, 0.25)
            .rounded_rectangle([320.0, 610.0], 220.0, 90.0, 18.0);
        ctx.color(0.2, 0.62, 0.95)
            .ellipse([430.0, 520.0], 90.0, 45.0);
        ctx.color(0.9, 0.5, 0.6)
            .circle([430.0, 420.0], 45.0);
    });

    Ok(())
}

fn add_page_png(
    doc: &mut Document,
    png_bytes: &[u8],
    width: u32,
    height: u32,
) -> PdfResult<()> {
    let (page_width, page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 36.0;
    let header_height = 32.0;
    doc.start_new_page();
    let (draw_x, draw_y, draw_width, draw_height) = fit_image(
        page_width,
        page_height,
        margin,
        header_height,
        width,
        height,
    );

    doc.font("Helvetica").size(14.0);
    doc.text_at("Page 2: embedded PNG", [margin, page_height - margin - 16.0]);
    doc.image_png(png_bytes, [draw_x, draw_y], draw_width, draw_height)?;
    Ok(())
}

fn add_page_jpeg(
    doc: &mut Document,
    jpeg_bytes: &[u8],
    width: u32,
    height: u32,
) -> PdfResult<()> {
    let (page_width, page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 36.0;
    let header_height = 32.0;
    doc.start_new_page();
    let (draw_x, draw_y, draw_width, draw_height) = fit_image(
        page_width,
        page_height,
        margin,
        header_height,
        width,
        height,
    );
    doc.font("Helvetica").size(14.0);
    doc.text_at("Page 3: embedded JPEG", [margin, page_height - margin - 16.0]);
    doc.image_jpeg(jpeg_bytes, [draw_x, draw_y], draw_width, draw_height)?;
    Ok(())
}

fn add_page_alpha(
    doc: &mut Document,
    alpha_bytes: &[u8],
    width: u32,
    height: u32,
) -> PdfResult<()> {
    let (page_width, page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 36.0;
    let header_height = 32.0;
    doc.start_new_page();
    let (draw_x, draw_y, draw_width, draw_height) = fit_image(
        page_width,
        page_height,
        margin,
        header_height,
        width,
        height,
    );
    doc.fill(|ctx| {
        ctx.color(0.92, 0.98, 0.92)
            .rectangle([0.0, 0.0], page_width, page_height);
    });
    doc.font("Helvetica").size(14.0);
    doc.text_at(
        "Page 4: example-alpha.png over green background",
        [margin, page_height - margin - 16.0],
    );
    doc.image_png(alpha_bytes, [draw_x, draw_y], draw_width, draw_height)?;
    Ok(())
}

fn read_png_dimensions(data: &[u8]) -> StdResult<(u32, u32), Box<dyn Error>> {
    let decoder = png::Decoder::new(Cursor::new(data));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    Ok((info.width, info.height))
}

fn fit_image(
    page_width: f64,
    page_height: f64,
    margin: f64,
    header_height: f64,
    image_width: u32,
    image_height: u32,
) -> (f64, f64, f64, f64) {
    let max_width = page_width - margin * 2.0;
    let max_height = page_height - margin * 2.0 - header_height;
    let image_aspect = image_width as f64 / image_height as f64;

    let mut draw_width = max_width;
    let mut draw_height = draw_width / image_aspect;
    if draw_height > max_height {
        draw_height = max_height;
        draw_width = draw_height * image_aspect;
    }

    let draw_x = (page_width - draw_width) / 2.0;
    let draw_y = margin + (max_height - draw_height) / 2.0;
    (draw_x, draw_y, draw_width, draw_height)
}
