//! CJK Font Support Demo
//!
//! Demonstrates pdfcrate's CJK (Chinese, Japanese, Korean) text rendering:
//! - Multiple font weights (light, regular, medium)
//! - Chinese text samples
//! - Mixed language content
//! - Classical Chinese text
//! - Japanese hiragana, katakana, and kanji
//! - Font size variations
//!
//! Run with: cargo run --example cjk_demo --features fonts

use pdfcrate::prelude::{Document, LayoutDocument, Margin, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;
use std::fs;

// CJK font (LXGW WenKai - 霞鹜文楷) - run `./examples/download-fonts.sh` first
const CJK_FONT_LIGHT: &str = "examples/fonts/LXGWWenKai-Light.ttf";
const CJK_FONT_REGULAR: &str = "examples/fonts/LXGWWenKai-Regular.ttf";
const CJK_FONT_MEDIUM: &str = "examples/fonts/LXGWWenKai-Medium.ttf";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("cjk_demo.pdf", |doc| {
        doc.title("CJK Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: cjk_demo.pdf");
    Ok(())
}

/// Adds the CJK font support demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    // Header band (top-left: 0, 842)
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 842.0], page_width, 82.0);
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

    // Wrap into LayoutDocument for cursor-based body
    // Prawn: margin 36, indent(12) → text at 48
    let doc_owned = std::mem::take(doc);
    let mut layout = LayoutDocument::with_margin(
        doc_owned,
        Margin::new(132.0, margin - 12.0, margin - 12.0, margin - 12.0),
    );

    let abs_bottom = layout.bounds().absolute_bottom();

    layout.indent(12.0, 0.0, |layout| {
        layout.font("Helvetica").size(14.0);
        layout.text("Chinese Text Samples:");
        layout.move_down(16.0);

        layout.font(&cjk_light).size(16.0);
        layout.text("Light: 霞鹜文楷是一款开源中文字体");
        layout.move_down(6.0);

        layout.font(&cjk_regular).size(16.0);
        layout.text("Regular: 天地玄黄，宇宙洪荒");
        layout.move_down(6.0);

        layout.font(&cjk_medium).size(16.0);
        layout.text("Medium: 日月盈昃，辰宿列张");
        layout.move_down(21.0);

        layout.font("Helvetica").size(14.0);
        layout.text("Mixed Language Content:");
        layout.move_down(16.0);

        layout.font(&cjk_regular).size(14.0);
        layout.text("PDF 库支持 TrueType 字体嵌入");
        layout.move_down(4.0);
        layout.text("支持字体子集化 (Font Subsetting) 以减小文件大小");
        layout.move_down(4.0);
        layout.text("开源的高性能 PDF 生成库");
        layout.move_down(22.0);

        layout.font("Helvetica").size(14.0);
        layout.text("Classical Chinese:");
        layout.move_down(16.0);

        layout.font(&cjk_regular).size(13.0);
        let classical_lines = [
            "子曰：「学而时习之，不亦说乎？",
            "有朋自远方来，不亦乐乎？",
            "人不知而不愠，不亦君子乎？」",
            "",
            "《论语·学而》",
        ];
        for line in classical_lines {
            layout.text(line);
            layout.move_down(4.0);
        }
        layout.move_down(16.0);

        layout.font("Helvetica").size(14.0);
        layout.text("Japanese Text:");
        layout.move_down(16.0);

        layout.font(&cjk_regular).size(14.0);
        layout.text("ひらがな: あいうえお かきくけこ");
        layout.move_down(4.0);
        layout.text("カタカナ: アイウエオ カキクケコ");
        layout.move_down(4.0);
        layout.text("漢字混じり: 東京は日本の首都です");
        layout.move_down(22.0);

        layout.font("Helvetica").size(14.0);
        layout.text("Size Variations:");
        layout.move_down(8.0);

        for size in [10.0, 14.0, 18.0, 24.0] {
            layout.font(&cjk_regular).size(size);
            layout.text(&format!("{}pt: 中文字体大小测试", size as i32));
        }
    });

    // Prawn: draw_text at: [12, 24] → absolute (36+12, 36+24) = (48, 60)
    layout.font("Helvetica").size(10.0);
    layout.text_at(
        "Note: CJK fonts are subsetted - only used glyphs are embedded.",
        [margin, abs_bottom + 24.0],
    );

    *doc = layout.into_inner();
    Ok(())
}
