"""
pdfcrate - A fast PDF generation library for Python

Example usage:

    from pdfcrate import Document, Margin, Color, TextAlign

    # Simple usage
    doc = Document()
    doc.font("Helvetica-Bold", 24).text_at("Hello", (100, 700)).save("out.pdf")

    # With context manager
    with Document("out.pdf") as doc:
        doc.font("Helvetica-Bold", 24)
        doc.text_at("Hello", (100, 700))

    # With layout (margin enables cursor-based layout)
    doc = Document(margin=Margin.all(72))
    doc.text("Hello")
    doc.move_down(20)
    doc.text_wrap("Long text...")
    doc.save("out.pdf")
"""

from .pdfcrate import (
    # Core document
    Document,
    # Types
    Color,
    Margin,
    PageSize,
    TextAlign,
    VerticalAlign,
    Overflow,
    TextBoxResult,
    # Context managers
    TransparentContext,
    IndentContext,
    FloatContext,
    FontContext,
    BoundingBoxContext,
    PageContext,
    # Table
    Table,
    # Rich text
    TextFragment,
    SpanBuilder,
    # Outline/Bookmarks
    OutlineItem,
    # Image
    EmbeddedImage,
    EmbeddedFont,
)

__all__ = [
    "Document",
    "Color",
    "Margin",
    "PageSize",
    "TextAlign",
    "VerticalAlign",
    "Overflow",
    "TextBoxResult",
    "TransparentContext",
    "IndentContext",
    "FloatContext",
    "FontContext",
    "BoundingBoxContext",
    "PageContext",
    "Table",
    "TextFragment",
    "SpanBuilder",
    "OutlineItem",
    "EmbeddedImage",
    "EmbeddedFont",
]

__version__ = "0.1.0"
