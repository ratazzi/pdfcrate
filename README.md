# pdfcrate

> **Warning**: This project is under active development. The API is unstable and subject to breaking changes. **Do not use in production environments.**

A Rust library for creating and manipulating PDF documents with a focus on ease of use and comprehensive text layout support.

## Features

- **Layout-based document creation** - Cursor-based layout with automatic text flow, margins, and bounding boxes
- **Rich text formatting** - Bold, italic, colors, alignment (left/center/right/justify)
- **Font support** - 14 standard PDF fonts + TrueType/OpenType embedding with text shaping
- **Font fallback** - Automatic fallback for mixed-language text (CJK, emoji, etc.)
- **Tables** - Full-featured tables with borders, cell styles, column spans, and overflow handling
- **Images** - PNG and JPEG embedding with fit/fill modes
- **SVG rendering** - Embed SVG graphics directly
- **Links and outlines** - Hyperlinks, internal links, and document bookmarks
- **Forms** - Interactive form fields (text, checkbox, radio, dropdown)
- **WASM support** - Works in WebAssembly environments (tested with Cloudflare Workers)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
pdfcrate = "0.1"
```

## Quick Start

```rust
use pdfcrate::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("hello.pdf", |doc| {
        doc.title("Hello PDF").author("pdfcrate");

        doc.font("Helvetica-Bold").size(24.0);
        doc.text("Hello, World!");

        doc.move_down(20.0);

        doc.font("Helvetica").size(12.0);
        doc.text_wrap("This is a simple PDF document created with pdfcrate. \
                       Text automatically wraps within the page margins.");

        Ok(())
    })?;

    Ok(())
}
```

## Examples

### Custom Fonts

```rust
use pdfcrate::prelude::*;

Document::generate("custom_font.pdf", |doc| {
    // Embed a TrueType font
    let font = doc.embed_font_file("fonts/MyFont.ttf")?;

    doc.font(&font).size(16.0);
    doc.text("Text with custom font!");

    Ok(())
})?;
```

### Font Fallback for CJK

```rust
use pdfcrate::prelude::*;

Document::generate("multilingual.pdf", |doc| {
    let cjk_font = doc.embed_font_file("fonts/NotoSansCJK.ttf")?;

    // Configure fallback fonts
    doc.fallback_fonts(vec![cjk_font]);

    doc.font("Helvetica").size(14.0);
    doc.text_wrap("English text mixed with 中文 and 日本語");

    Ok(())
})?;
```

### Tables

```rust
use pdfcrate::prelude::*;

Document::generate("table.pdf", |doc| {
    let table = Table::new(&[100.0, 150.0, 100.0])
        .header(&["Name", "Description", "Price"])
        .row(&["Item 1", "First item", "$10.00"])
        .row(&["Item 2", "Second item", "$20.00"])
        .borders(BorderStyle::all(0.5, Color::BLACK));

    doc.table(&table);

    Ok(())
})?;
```

### Images

```rust
use pdfcrate::prelude::*;

Document::generate("with_image.pdf", |doc| {
    // Embed and draw image
    doc.image_fit("photo.png", [0.0, 0.0], 200.0, 150.0)?;

    Ok(())
})?;
```

## Feature Flags

All features are pure Rust and WASM-compatible.

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support (file I/O) |
| `png` | Yes | PNG image support (JPEG is always supported) |
| `fonts` | Yes | TrueType/OpenType font embedding |
| `text-shaping` | Yes | Complex text shaping via rustybuzz |
| `svg` | Yes | SVG rendering support |
| `barcode` | Yes | QR code and barcode generation |

To use minimal features:

```toml
[dependencies]
pdfcrate = { version = "0.1", default-features = false }
```

## Minimum Supported Rust Version (MSRV)

Rust 1.70 or later.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
