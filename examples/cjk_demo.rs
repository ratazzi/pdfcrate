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

use pdfcrate::prelude::{Document, PageLayout, PageSize};
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
        ctx.gray(0.95).rect_tl([0.0, 842.0], page_width, 82.0);
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
    doc.text_at("PDF 库 pdfcrate 支持 TrueType 字体嵌入", [margin, y]);
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
