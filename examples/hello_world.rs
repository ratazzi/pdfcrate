//! Hello World PDF example
//!
//! Run with: cargo run --example hello_world

use pdfcrate::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Method 1: Using generate with closure
    Document::generate("hello_world.pdf", |doc| {
        doc.title("Hello World PDF").author("pdfcrate");

        doc.font("Helvetica").size(24.0);
        doc.text_at("Hello, World!", [72.0, 700.0]);

        doc.font("Times-Roman").size(12.0);
        doc.text_at("This PDF was created using pdfcrate.", [72.0, 650.0]);
        doc.text_at("A Rust library for creating PDF documents.", [72.0, 630.0]);

        // Draw some shapes
        doc.stroke(|ctx| {
            ctx.color(1.0, 0.0, 0.0); // Red
            ctx.line_width(2.0);
            ctx.rectangle([72.0, 500.0], 100.0, 50.0);
        });

        doc.fill(|ctx| {
            ctx.color(0.0, 0.0, 1.0); // Blue
            ctx.rectangle([200.0, 500.0], 100.0, 50.0);
        });

        // Second page
        doc.start_new_page();
        doc.font("Courier").size(14.0);
        doc.text_at("This is page 2!", [72.0, 700.0]);

        Ok(())
    })?;

    println!("Created: hello_world.pdf");

    // Method 2: Using imperative style
    let mut doc = Document::new();
    doc.title("Simple PDF");
    doc.text_at("Simple document using imperative style.", [72.0, 700.0]);
    doc.save("simple.pdf")?;

    println!("Created: simple.pdf");

    Ok(())
}
