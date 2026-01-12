//! Showcase PDF demonstrating pdf_rs features:
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
//!
//! Run with: cargo run --example showcase --features "fonts,text-shaping,svg"

use pdf_rs::image::embed_jpeg;
use pdf_rs::prelude::{Document, LayoutDocument, LoadedDocument, Margin, PageLayout, PageSize};
use pdf_rs::Result as PdfResult;
use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::result::Result as StdResult;

// Path to a TrueType font for the custom font demo page
const FONT_PATH: &str = "/Users/ratazzi/Downloads/Roboto_v3.014/web/static/Roboto-Regular.ttf";
const FONT_BOLD_PATH: &str = "/Users/ratazzi/Downloads/Roboto_v3.014/web/static/Roboto-Bold.ttf";
const FONT_ITALIC_PATH: &str =
    "/Users/ratazzi/Downloads/Roboto_v3.014/web/static/Roboto-Italic.ttf";
const MAPLE_FONT_PATH: &str = "/Users/ratazzi/Library/Fonts/MapleMono-NF-CN-Regular.ttf";

// CJK font (LXGW WenKai - 霞鹜文楷)
const CJK_FONT_LIGHT: &str = "/Users/ratazzi/Library/Fonts/LXGWWenKai-Light.ttf";
const CJK_FONT_REGULAR: &str = "/Users/ratazzi/Library/Fonts/LXGWWenKai-Regular.ttf";
const CJK_FONT_MEDIUM: &str = "/Users/ratazzi/Library/Fonts/LXGWWenKai-Medium.ttf";

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

        #[cfg(feature = "fonts")]
        add_page_custom_font(doc)?;

        #[cfg(feature = "fonts")]
        add_page_ligatures(doc)?;

        #[cfg(feature = "fonts")]
        add_page_cjk(doc)?;

        add_page_forms(doc)?;
        add_page_pdf_embed(doc)?;

        #[cfg(feature = "svg")]
        add_page_svg_barcode(doc)?;

        add_page_layout(doc)?;

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
    doc.text_at("Drawing primitives", [48.0, 784.0]);

    doc.font("Helvetica").size(12.0);
    doc.text_at("Strokes", [60.0, 720.0]);
    doc.text_at("Fills", [320.0, 720.0]);

    // Stroke-only shapes
    doc.stroke(|ctx| {
        ctx.color(0.15, 0.45, 0.85)
            .line_width(2.0)
            .rectangle([60.0, 610.0], 180.0, 90.0);
        ctx.color(0.9, 0.3, 0.2).line_width(3.0).rounded_rectangle(
            [60.0, 490.0],
            180.0,
            90.0,
            14.0,
        );
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
        ctx.color(0.9, 0.5, 0.6).circle([430.0, 420.0], 45.0);
    });

    Ok(())
}

fn add_page_png(doc: &mut Document, png_bytes: &[u8], width: u32, height: u32) -> PdfResult<()> {
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
    doc.text_at("Embedded PNG", [margin, page_height - margin - 16.0]);
    doc.image_png(png_bytes, [draw_x, draw_y], draw_width, draw_height)?;
    Ok(())
}

fn add_page_jpeg(doc: &mut Document, jpeg_bytes: &[u8], width: u32, height: u32) -> PdfResult<()> {
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
    doc.text_at("Embedded JPEG", [margin, page_height - margin - 16.0]);
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
        "PNG with alpha over green background",
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

#[cfg(feature = "fonts")]
fn add_page_custom_font(doc: &mut Document) -> PdfResult<()> {
    use std::fs;

    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    doc.start_new_page();

    // Header band (light gray)
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 760.0], page_width, 82.0);
    });

    // Embed fonts
    let font_regular = doc.embed_font(fs::read(FONT_PATH)?)?;
    let font_bold = doc.embed_font(fs::read(FONT_BOLD_PATH)?)?;
    let font_italic = doc.embed_font(fs::read(FONT_ITALIC_PATH)?)?;

    // Page title (using embedded font)
    doc.font(&font_bold).size(28.0);
    doc.text_at("Custom Font Embedding", [margin, 800.0]);

    doc.font(&font_regular).size(12.0);
    doc.text_at(
        "TrueType fonts with full Unicode support",
        [margin, 778.0],
    );

    // Section 1: Font showcase
    let mut y = 700.0;

    doc.font("Helvetica").size(14.0);
    doc.text_at("Font Comparison:", [margin, y]);
    y -= 30.0;

    // Standard font
    doc.font("Helvetica").size(16.0);
    doc.text_at(
        "Helvetica (Standard): The quick brown fox jumps over the lazy dog.",
        [margin, y],
    );
    y -= 25.0;

    // Embedded fonts
    doc.font(&font_regular).size(16.0);
    doc.text_at(
        "Roboto Regular: The quick brown fox jumps over the lazy dog.",
        [margin, y],
    );
    y -= 25.0;

    doc.font(&font_bold).size(16.0);
    doc.text_at(
        "Roboto Bold: The quick brown fox jumps over the lazy dog.",
        [margin, y],
    );
    y -= 25.0;

    doc.font(&font_italic).size(16.0);
    doc.text_at(
        "Roboto Italic: The quick brown fox jumps over the lazy dog.",
        [margin, y],
    );
    y -= 45.0;

    // Section 2: Size variations
    doc.font("Helvetica").size(14.0);
    doc.text_at("Size Variations:", [margin, y]);
    y -= 25.0;

    for size in [10.0, 14.0, 18.0, 24.0, 32.0] {
        doc.font(&font_regular).size(size);
        doc.text_at(&format!("{}pt: Roboto Font", size as i32), [margin, y]);
        y -= size + 8.0;
    }
    y -= 20.0;

    // Section 3: Text measurement
    doc.font("Helvetica").size(14.0);
    doc.text_at("Text Measurement:", [margin, y]);
    y -= 30.0;

    doc.font(&font_regular).size(18.0);
    let sample_text = "Measured Text Width";
    let text_width = doc.measure_text(sample_text);

    // Draw the text
    doc.text_at(sample_text, [margin, y]);

    // Draw a line under it showing the measured width
    doc.stroke(|ctx| {
        ctx.color(0.9, 0.2, 0.2)
            .line_width(2.0)
            .line([margin, y - 5.0], [margin + text_width, y - 5.0]);
    });

    doc.font(&font_regular).size(11.0);
    doc.text_at(
        &format!("Width: {:.1} points at 18pt", text_width),
        [margin, y - 20.0],
    );
    y -= 60.0;

    // Section 4: Mixed content
    doc.font("Helvetica").size(14.0);
    doc.text_at("Mixed Fonts in Document:", [margin, y]);
    y -= 30.0;

    // Draw a box with mixed font content
    doc.stroke(|ctx| {
        ctx.gray(0.7).line_width(1.0).rounded_rectangle(
            [margin, y - 80.0],
            page_width - margin * 2.0,
            90.0,
            8.0,
        );
    });

    doc.font(&font_bold).size(14.0);
    doc.text_at("Note:", [margin + 15.0, y - 10.0]);

    doc.font(&font_regular).size(12.0);
    doc.text_at(
        "This PDF demonstrates seamless mixing of standard PDF fonts",
        [margin + 15.0, y - 30.0],
    );
    doc.text_at(
        "(Helvetica, Times, Courier) with embedded TrueType fonts (Roboto).",
        [margin + 15.0, y - 45.0],
    );
    doc.text_at(
        "Text is fully searchable and can be copied from the PDF.",
        [margin + 15.0, y - 60.0],
    );

    Ok(())
}

#[cfg(feature = "fonts")]
fn add_page_ligatures(doc: &mut Document) -> PdfResult<()> {
    use std::fs;

    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    doc.start_new_page();

    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 760.0], page_width, 82.0);
    });

    let font_name = doc.embed_font(fs::read(MAPLE_FONT_PATH)?)?;

    doc.font(&font_name).size(28.0);
    doc.text_at("MapleMono Ligatures", [margin, 800.0]);

    doc.font("Helvetica").size(12.0);
    doc.text_at(
        "Ligatures, kerning, and line spacing",
        [margin, 778.0],
    );

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

    let roboto_font = doc.embed_font(fs::read(FONT_PATH)?)?;
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

#[cfg(feature = "fonts")]
fn add_page_cjk(doc: &mut Document) -> PdfResult<()> {
    use std::fs;

    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    doc.start_new_page();

    // Header band
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 760.0], page_width, 82.0);
    });

    // Embed CJK fonts
    let cjk_light = doc.embed_font(fs::read(CJK_FONT_LIGHT)?)?;
    let cjk_regular = doc.embed_font(fs::read(CJK_FONT_REGULAR)?)?;
    let cjk_medium = doc.embed_font(fs::read(CJK_FONT_MEDIUM)?)?;

    // Page title
    doc.font(&cjk_medium).size(28.0);
    doc.text_at("CJK 字体支持", [margin, 800.0]);

    doc.font(&cjk_regular).size(12.0);
    doc.text_at(
        "中日韩文字渲染 / Chinese, Japanese, Korean",
        [margin, 778.0],
    );

    let mut y = 710.0;

    // Section 1: Chinese text samples
    doc.font("Helvetica").size(14.0);
    doc.text_at("Chinese Text Samples:", [margin, y]);
    y -= 30.0;

    doc.font(&cjk_light).size(16.0);
    doc.text_at("Light: 霞鹜文楷是一款开源中文字体", [margin, y]);
    y -= 25.0;

    doc.font(&cjk_regular).size(16.0);
    doc.text_at("Regular: 天地玄黄，宇宙洪荒", [margin, y]);
    y -= 25.0;

    doc.font(&cjk_medium).size(16.0);
    doc.text_at("Medium: 日月盈昃，辰宿列张", [margin, y]);
    y -= 40.0;

    // Section 2: Mixed content
    doc.font("Helvetica").size(14.0);
    doc.text_at("Mixed Language Content:", [margin, y]);
    y -= 30.0;

    doc.font(&cjk_regular).size(14.0);
    doc.text_at("PDF 库 pdf.rs 支持 TrueType 字体嵌入", [margin, y]);
    y -= 22.0;
    doc.text_at(
        "支持字体子集化 (Font Subsetting) 以减小文件大小",
        [margin, y],
    );
    y -= 22.0;
    doc.text_at("可用于 Cloudflare Workers 等 WASM 环境", [margin, y]);
    y -= 40.0;

    // Section 3: Classical Chinese
    doc.font("Helvetica").size(14.0);
    doc.text_at("Classical Chinese:", [margin, y]);
    y -= 30.0;

    doc.font(&cjk_regular).size(13.0);
    let classical_lines = [
        "子曰：「学而时习之，不亦说乎？",
        "有朋自远方来，不亦乐乎？",
        "人不知而不愠，不亦君子乎？」",
        "",
        "《论语·学而》",
    ];
    for line in classical_lines {
        doc.text_at(line, [margin, y]);
        y -= 20.0;
    }
    y -= 20.0;

    // Section 4: Japanese text
    doc.font("Helvetica").size(14.0);
    doc.text_at("Japanese Text:", [margin, y]);
    y -= 30.0;

    doc.font(&cjk_regular).size(14.0);
    doc.text_at("ひらがな: あいうえお かきくけこ", [margin, y]);
    y -= 22.0;
    doc.text_at("カタカナ: アイウエオ カキクケコ", [margin, y]);
    y -= 22.0;
    doc.text_at("漢字混じり: 東京は日本の首都です", [margin, y]);
    y -= 40.0;

    // Section 5: Size variations with CJK
    doc.font("Helvetica").size(14.0);
    doc.text_at("Size Variations:", [margin, y]);
    y -= 25.0;

    for size in [10.0, 14.0, 18.0, 24.0] {
        doc.font(&cjk_regular).size(size);
        doc.text_at(&format!("{}pt: 中文字体大小测试", size as i32), [margin, y]);
        y -= size + 8.0;
    }

    // Footer note
    doc.font("Helvetica").size(10.0);
    doc.text_at(
        "Note: CJK fonts are subsetted - only used glyphs are embedded to reduce file size.",
        [margin, 60.0],
    );

    Ok(())
}

fn add_page_forms(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    doc.start_new_page();

    // Header band
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 760.0], page_width, 82.0);
    });

    doc.font("Helvetica").size(24.0);
    doc.text_at("Interactive Forms", [margin, 800.0]);

    doc.font("Helvetica").size(11.0);
    doc.text_at(
        "AcroForms - text fields, checkboxes, and dropdowns",
        [margin, 780.0],
    );

    let mut y = 700.0;
    let label_x = margin;
    let field_x = margin + 120.0;
    let field_width = 200.0;
    let field_height = 20.0;
    let row_height = 35.0;

    // Section: Contact Information
    doc.font("Helvetica").size(14.0);
    doc.text_at("Contact Information", [label_x, y]);
    y -= row_height;

    doc.font("Helvetica").size(11.0);

    // Name field
    doc.text_at("Name:", [label_x, y + 5.0]);
    doc.add_text_field(
        "name",
        [field_x, y, field_x + field_width, y + field_height],
    );
    y -= row_height;

    // Email field
    doc.text_at("Email:", [label_x, y + 5.0]);
    doc.add_text_field(
        "email",
        [field_x, y, field_x + field_width, y + field_height],
    );
    y -= row_height;

    // Phone field
    doc.text_at("Phone:", [label_x, y + 5.0]);
    doc.add_text_field("phone", [field_x, y, field_x + 150.0, y + field_height]);
    y -= row_height + 20.0;

    // Section: Preferences
    doc.font("Helvetica").size(14.0);
    doc.text_at("Preferences", [label_x, y]);
    y -= row_height;

    doc.font("Helvetica").size(11.0);

    // Newsletter checkbox
    doc.text_at("Subscribe:", [label_x, y + 5.0]);
    doc.add_checkbox("newsletter", [field_x, y, field_x + 18.0, y + 18.0], true);
    doc.text_at("Newsletter", [field_x + 25.0, y + 5.0]);
    y -= row_height;

    // Updates checkbox
    doc.text_at("Receive:", [label_x, y + 5.0]);
    doc.add_checkbox("updates", [field_x, y, field_x + 18.0, y + 18.0], false);
    doc.text_at("Product updates", [field_x + 25.0, y + 5.0]);
    y -= row_height + 20.0;

    // Section: Selection
    doc.font("Helvetica").size(14.0);
    doc.text_at("Selection", [label_x, y]);
    y -= row_height;

    doc.font("Helvetica").size(11.0);

    // Country dropdown
    doc.text_at("Country:", [label_x, y + 5.0]);
    doc.add_dropdown(
        "country",
        [field_x, y, field_x + field_width, y + field_height],
        vec!["USA", "Canada", "UK", "Germany", "France", "Japan"],
    );
    y -= row_height;

    // Department dropdown
    doc.text_at("Department:", [label_x, y + 5.0]);
    doc.add_dropdown(
        "department",
        [field_x, y, field_x + 150.0, y + field_height],
        vec!["Sales", "Engineering", "Marketing", "Support"],
    );

    // Footer note
    doc.font("Helvetica").size(10.0);
    doc.text_at(
        "Note: Form fields are interactive - click to edit in a PDF viewer.",
        [margin, 80.0],
    );

    let field_count = doc.form_field_count();
    doc.text_at(
        &format!("Total form fields: {}", field_count),
        [margin, 60.0],
    );

    Ok(())
}

fn add_page_pdf_embed(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    doc.start_new_page();

    // Header band
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 760.0], page_width, 82.0);
    });

    doc.font("Helvetica").size(24.0);
    doc.text_at("PDF Embedding", [margin, 800.0]);

    doc.font("Helvetica").size(11.0);
    doc.text_at(
        "Embed and draw pages from other PDFs",
        [margin, 780.0],
    );

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
        // Draw a border around the thumbnail
        doc.stroke(|ctx| {
            ctx.gray(0.7).line_width(1.0).rectangle(
                [x - 2.0, y - thumbnail_height - 2.0],
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

#[cfg(feature = "svg")]
fn add_page_svg_barcode(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    doc.start_new_page();

    // Header band
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 760.0], page_width, 82.0);
    });

    doc.font("Helvetica").size(24.0);
    doc.text_at("SVG Barcode", [margin, 800.0]);

    doc.font("Helvetica").size(11.0);
    doc.text_at("SVG rendering", [margin, 780.0]);

    doc.font("Helvetica").size(12.0);
    doc.text_at(
        "The barcode below is drawn from SVG (basic shapes are converted to paths).",
        [margin, 720.0],
    );

    let barcode_svg = r##"
<svg xmlns="http://www.w3.org/2000/svg" width="220" height="80" viewBox="0 0 220 80">
  <rect x="8" y="8" width="4" height="64" fill="#000"/>
  <rect x="16" y="8" width="2" height="64" fill="#000"/>
  <rect x="22" y="8" width="6" height="64" fill="#000"/>
  <rect x="32" y="8" width="2" height="64" fill="#000"/>
  <rect x="38" y="8" width="4" height="64" fill="#000"/>
  <rect x="46" y="8" width="2" height="64" fill="#000"/>
  <rect x="52" y="8" width="6" height="64" fill="#000"/>
  <rect x="62" y="8" width="2" height="64" fill="#000"/>
  <rect x="68" y="8" width="4" height="64" fill="#000"/>
  <rect x="76" y="8" width="2" height="64" fill="#000"/>
  <rect x="82" y="8" width="6" height="64" fill="#000"/>
  <rect x="92" y="8" width="2" height="64" fill="#000"/>
  <rect x="98" y="8" width="4" height="64" fill="#000"/>
  <rect x="106" y="8" width="2" height="64" fill="#000"/>
  <rect x="112" y="8" width="6" height="64" fill="#000"/>
  <rect x="122" y="8" width="2" height="64" fill="#000"/>
  <rect x="128" y="8" width="4" height="64" fill="#000"/>
  <rect x="136" y="8" width="2" height="64" fill="#000"/>
  <rect x="142" y="8" width="6" height="64" fill="#000"/>
  <rect x="152" y="8" width="2" height="64" fill="#000"/>
  <rect x="158" y="8" width="4" height="64" fill="#000"/>
  <rect x="166" y="8" width="2" height="64" fill="#000"/>
  <rect x="172" y="8" width="6" height="64" fill="#000"/>
  <rect x="182" y="8" width="2" height="64" fill="#000"/>
  <rect x="188" y="8" width="4" height="64" fill="#000"/>
  <rect x="196" y="8" width="2" height="64" fill="#000"/>
  <rect x="202" y="8" width="6" height="64" fill="#000"/>
</svg>
"##;

    let target_width = page_width - margin * 2.0;
    let target_height = 140.0;
    let x = margin;
    let y = 520.0;

    doc.fill(|ctx| {
        ctx.gray(0.97).rectangle(
            [x - 8.0, y - 8.0],
            target_width + 16.0,
            target_height + 16.0,
        );
    });
    doc.stroke(|ctx| {
        ctx.gray(0.85).line_width(0.5).rectangle(
            [x - 8.0, y - 8.0],
            target_width + 16.0,
            target_height + 16.0,
        );
    });

    doc.draw_svg(barcode_svg, [x, y], target_width, target_height)?;

    doc.font("Helvetica").size(10.0);
    doc.text_at(
        "Use SVG for barcodes, charts, and icons without rasterization.",
        [margin, 470.0],
    );

    Ok(())
}

fn add_page_layout(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    doc.start_new_page();

    // Header band (drawn with absolute positioning)
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 760.0], page_width, 82.0);
    });

    doc.font("Helvetica").size(24.0);
    doc.text_at("LayoutDocument Demo", [margin, 800.0]);

    doc.font("Helvetica").size(11.0);
    doc.text_at(
        "Prawn-style cursor-based layout (no manual coordinate calculation)",
        [margin, 780.0],
    );

    // Create LayoutDocument wrapper
    let doc_owned = std::mem::take(doc);
    let mut layout =
        LayoutDocument::with_margin(doc_owned, Margin::new(82.0, margin, margin, margin));

    // Section 1: Bounding Box Demo
    layout.font("Helvetica").size(14.0);
    layout.text("Nested Bounding Boxes:");
    layout.move_down(15.0);

    // Outer box (fixed height)
    layout.bounding_box([0.0, 0.0], 250.0, Some(120.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(11.0);
        doc.text("Outer box (250x120)");

        // Inner box (nested, stretchy)
        doc.bounding_box([20.0, 0.0], 180.0, None, |doc| {
            doc.stroke_bounds();
            doc.font("Helvetica").size(10.0);
            doc.text("Inner nested box");
            doc.text("Auto-height (stretchy)");
        });
    });

    // Side-by-side boxes using float
    layout.move_down(20.0);
    layout.font("Helvetica").size(14.0);
    layout.text("Side-by-Side Layout (using float):");
    layout.move_down(15.0);

    let box_top = layout.cursor();

    // Left box
    layout.bounding_box([0.0, 0.0], 160.0, Some(80.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(10.0);
        doc.text("Left Box");
        doc.text("Width: 160pt");
    });

    // Right box (use float to position at same y level)
    layout.set_cursor(box_top);
    layout.bounding_box([180.0, 0.0], 160.0, Some(80.0), |doc| {
        doc.stroke_bounds();
        doc.font("Helvetica").size(10.0);
        doc.text("Right Box");
        doc.text("Offset: 180pt");
    });

    layout.move_down(20.0);

    // Section 2: Cursor tracking
    layout.font("Helvetica").size(14.0);
    layout.text("Cursor Tracking:");
    layout.move_down(10.0);

    layout.font("Helvetica").size(11.0);
    let cursor1 = layout.cursor();
    layout.text(&format!("Cursor at: {:.1}pt", cursor1));
    layout.text("Each text() call auto-advances cursor");
    let cursor2 = layout.cursor();
    layout.text(&format!(
        "Now at: {:.1}pt (moved {:.1}pt)",
        cursor2,
        cursor1 - cursor2
    ));

    layout.move_down(20.0);

    // Section 3: Indent
    layout.font("Helvetica").size(14.0);
    layout.text("Indent:");
    layout.move_down(10.0);

    layout.font("Helvetica").size(11.0);
    layout.text("Normal margin.");
    layout.indent(40.0, 40.0, |l| {
        l.text("Indented 40pt left/right.");
        l.indent(30.0, 30.0, |l| {
            l.text("Double indent (70pt total).");
        });
        l.text("Back to 40pt indent.");
    });
    layout.text("Back to normal.");

    layout.move_down(20.0);

    // Section 4: Float
    layout.font("Helvetica").size(14.0);
    layout.text("Float (temp position):");
    layout.move_down(10.0);

    layout.font("Helvetica").size(11.0);
    layout.text("Before float");
    layout.float(|l| {
        l.move_down(60.0);
        l.font("Helvetica").size(10.0);
        l.text(">> Floated 60pt down");
    });
    layout.text("After float (continues from 'Before')");
    layout.move_down(70.0); // Make room

    // Section 5: Full bounds visualization
    layout.font("Helvetica").size(10.0);
    layout.text("Current bounds shown by stroke_bounds():");
    layout.stroke_bounds();

    *doc = layout.into_inner();
    Ok(())
}

/// Creates a sample source PDF with multiple pages for embedding demonstration
fn create_sample_source_pdf() -> PdfResult<Vec<u8>> {
    let mut source = Document::new();

    // Page 1: Title page
    source.fill(|ctx| {
        ctx.color(0.2, 0.4, 0.8)
            .rectangle([0.0, 700.0], 595.0, 142.0);
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
        ctx.color(0.3, 0.9, 0.3)
            .rectangle([280.0, 420.0], 160.0, 160.0);
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
