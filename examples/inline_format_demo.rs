//! Inline Format Demo
//!
//! Demonstrates pdfcrate's HTML-like inline formatting:
//! - Bold, italic, underline, strikethrough
//! - Superscript and subscript
//! - Color and font changes
//! - Hyperlinks
//! - Line breaks (<br>)
//! - Automatic word wrapping with mixed styles
//!
//! Run with: cargo run --example inline_format_demo

use pdfcrate::api::Color;
use pdfcrate::prelude::{Document, LayoutDocument, Margin, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("inline_format_demo.pdf", |doc| {
        doc.title("Inline Format Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: inline_format_demo.pdf");
    Ok(())
}

/// Adds the inline format demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _) = PageSize::A4.dimensions(PageLayout::Portrait);

    // Header band
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 842.0], page_width, 82.0);
    });
    doc.font("Helvetica").size(24.0);
    doc.text_at("Inline Format Demo", [36.0, 800.0]);
    doc.font("Helvetica").size(11.0);
    doc.text_at(
        "HTML-like inline formatting with <b>, <i>, <u>, <color>, <font>, etc.",
        [36.0, 780.0],
    );

    let doc_owned = std::mem::take(doc);
    let mut layout = LayoutDocument::with_margin(doc_owned, Margin::new(136.0, 36.0, 36.0, 36.0));

    // Section 1: Basic bold/italic
    layout.font("Helvetica").size(12.0);
    layout.text("1. Basic Formatting (text_inline)");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text_inline("Normal, <b>bold</b>, <i>italic</i>, and <b><i>bold-italic</i></b>.");
    layout.move_down(4.0);
    layout.text_inline("<strong>Strong</strong> and <em>emphasis</em> tags also work.");

    layout.move_down(15.0);

    // Section 2: Underline / strikethrough
    layout.font("Helvetica").size(12.0);
    layout.text("2. Decorations");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text_inline("This text has <u>underlined words</u> and <strikethrough>struck-through words</strikethrough>.");

    layout.move_down(15.0);

    // Section 3: Superscript / subscript
    layout.font("Helvetica").size(12.0);
    layout.text("3. Superscript & Subscript");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text_inline("Einstein: E = mc<sup>2</sup>");
    layout.move_down(4.0);
    layout.text_inline("Water: H<sub>2</sub>O");
    layout.move_down(4.0);
    layout.text_inline("Quadratic: x<sup>2</sup> + 2x + 1 = (x+1)<sup>2</sup>");

    layout.move_down(15.0);

    // Section 4: Color
    layout.font("Helvetica").size(12.0);
    layout.text("4. Inline Color");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text_inline(
        r##"Traffic light: <color rgb="#FF0000">Red</color>, <color rgb="#FFA500">Amber</color>, <color rgb="#008000">Green</color>."##,
    );

    layout.move_down(15.0);

    // Section 5: Font changes
    layout.font("Helvetica").size(12.0);
    layout.text("5. Inline Font Changes");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text_inline(
        r#"Default, <font name="Courier">monospace</font>, <font name="Times-Roman">serif</font>, and <font name="Helvetica" size="14">larger</font>."#,
    );

    layout.move_down(15.0);

    // Section 6: Links
    layout.font("Helvetica").size(12.0);
    layout.text("6. Hyperlinks");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text_inline(
        r#"Visit <a href="https://www.rust-lang.org">Rust</a> or <link href="https://github.com">GitHub</link>."#,
    );

    layout.move_down(15.0);

    // Section 7: Line breaks
    layout.font("Helvetica").size(12.0);
    layout.text("7. Line Breaks");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text_inline("First line<br>Second line<br/>Third line");

    layout.move_down(15.0);

    // Section 8: HTML entities
    layout.font("Helvetica").size(12.0);
    layout.text("8. HTML Entities");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text_inline("Compare: 1 &lt; 2 &amp; 3 &gt; 2");

    layout.move_down(15.0);

    // Section 9: Auto-wrapping (text_wrap_inline)
    layout.font("Helvetica").size(12.0);
    layout.text("9. Auto-wrapping with Mixed Styles (text_wrap_inline)");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text_wrap_inline(
        "This is a <b>long paragraph</b> that demonstrates <i>automatic word wrapping</i> \
         with inline formatting. The text will flow across multiple lines while \
         preserving <u>underline</u> and <b><i>bold-italic</i></b> styles. \
         Even <font name=\"Courier\">monospace segments</font> wrap correctly.",
    );

    layout.move_down(15.0);

    // Section 10: Complex nested
    layout.font("Helvetica").size(12.0);
    layout.text("10. Complex Nested Markup");
    layout.move_down(8.0);

    layout.font("Helvetica").size(10.0);
    layout.text_inline(
        r##"<b>Bold with <i>italic inside</i> and <color rgb="#0000FF"><u>blue underline</u></color></b> then normal."##,
    );

    // Section 11: Horizontal rule separator
    layout.move_down(15.0);
    let rule_y = layout.cursor();
    layout.stroke(|ctx| {
        ctx.gray(0.7)
            .line_width(0.5)
            .line([36.0, rule_y], [page_width - 36.0, rule_y]);
    });
    layout.move_down(10.0);

    layout.font("Helvetica").size(9.0);
    layout
        .fill_color(Color::gray(0.4))
        .text("Generated by pdfcrate inline format engine");

    *doc = layout.into_inner();
    Ok(())
}
