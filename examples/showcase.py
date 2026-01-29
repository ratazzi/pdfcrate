#!/usr/bin/env python3
"""
Python Showcase for pdfcrate

Comprehensive demonstration matching Rust showcase.rs and Ruby showcase.rb exactly.
Uses the same coordinates and layout for comparison and validation.

Run with: python examples/showcase.py
"""

import io
import os
from pdfcrate import (
    Document,
    Margin,
    Color,
    TextAlign,
    TextFragment,
    OutlineItem,
    Overflow,
    TableOptions,
)

# Page dimensions (A4)
PAGE_WIDTH = 595.0
PAGE_HEIGHT = 842.0


def create_showcase():
    """Create comprehensive PDF showcase matching Rust and Ruby versions."""

    # Use 36pt margin to match Prawn default
    doc = Document(margin=Margin(36, 36, 36, 36))
    doc.title("pdfcrate Python Showcase").author("pdfcrate")

    # Track page indices for outline
    page_idx = 0
    drawing_page = page_idx

    # Drawing Primitives, Polygons & Transparency

    # Draw coordinate axes for visual reference (gray)
    doc.stroke_axis(at=(20, 20), color=(0.6, 0.6, 0.6), step=100)

    # Header band (top-left: 0, 842)
    doc.fill(Color.gray(0.95))
    doc.rect((0, PAGE_HEIGHT), PAGE_WIDTH, 82)

    doc.font("Helvetica", 24)
    doc.text_at("Drawing Primitives", (48, 804))
    doc.font("Helvetica", 11)
    doc.text_at("Strokes, fills, polygons & transparency", (48, 784))

    # Section labels
    doc.font("Helvetica", 12)
    doc.text_at("Strokes", (60, 720))
    doc.text_at("Fills", (320, 720))

    # Stroke-only shapes

    # Blue rectangle stroke (top-left: 60, 700), 180x90
    doc.stroke(Color(0.15, 0.45, 0.85), 2.0)
    doc.rect((60, 700), 180, 90, do_fill=False, do_stroke=True)

    # Red rounded rectangle stroke (top-left: 60, 580), 180x90, corner 14
    doc.stroke(Color(0.9, 0.3, 0.2), 3.0)
    doc.rounded_rect((60, 580), 180, 90, 14, do_fill=False, do_stroke=True)

    # Green circle stroke (center: 150, 420), radius 40
    doc.stroke(Color(0.2, 0.7, 0.4), 2.5)
    doc.circle((150, 420), 40, do_fill=False, do_stroke=True)

    # Dashed line
    doc.stroke(Color(0.2, 0.2, 0.2), 1.5)
    doc.dash([6.0, 4.0])
    doc.line((60, 360), (240, 360))
    doc.undash()

    # Filled shapes

    # Yellow rounded rectangle fill (top-left: 320, 700), 220x90, corner 18
    doc.fill(Color(0.98, 0.85, 0.25))
    doc.rounded_rect((320, 700), 220, 90, 18)

    # Blue ellipse fill (center: 430, 520), rx=90, ry=45
    doc.fill(Color(0.2, 0.62, 0.95))
    doc.ellipse((430, 520), 90, 45)

    # Pink circle fill (center: 430, 420), radius 45
    doc.fill(Color(0.9, 0.5, 0.6))
    doc.circle((430, 420), 45)

    # Polygons
    doc.font("Helvetica", 12)
    doc.text_at("Polygons", (60, 320))

    # Triangle (stroke)
    doc.stroke(Color(0.8, 0.2, 0.2), 2.5)
    doc.polygon([(100, 280), (140, 280), (120, 240)], do_fill=False, do_stroke=True)

    # Pentagon (fill)
    doc.fill(Color(0.2, 0.6, 0.8))
    doc.polygon([(200, 280), (220, 270), (215, 245), (185, 245), (180, 270)])

    # Star (fill)
    doc.fill(Color(0.9, 0.8, 0.2))
    doc.polygon([
        (310, 280), (315, 265), (330, 265), (320, 255), (325, 240),
        (310, 248), (295, 240), (300, 255), (290, 265), (305, 265)
    ])

    # Hexagon (transparent fill + stroke)
    with doc.transparent(0.6):
        doc.fill(Color(0.5, 0.3, 0.8))
        doc.polygon([(430, 280), (450, 270), (450, 250), (430, 240), (410, 250), (410, 270)])

    doc.stroke(Color(0.3, 0.1, 0.5), 2.0)
    doc.polygon([(430, 280), (450, 270), (450, 250), (430, 240), (410, 250), (410, 270)],
                do_fill=False, do_stroke=True)

    # Transparency
    doc.font("Helvetica", 12)
    doc.text_at("Transparency", (60, 200))

    # Overlapping circles with transparency
    circle_cx = 120.0
    circle_cy = 130.0

    # Red circle (100%)
    doc.fill(Color(1.0, 0.0, 0.0))
    doc.circle((circle_cx, circle_cy), 35)

    # Green circle (70%)
    with doc.transparent(0.7):
        doc.fill(Color(0.0, 1.0, 0.0))
        doc.circle((circle_cx + 40, circle_cy), 35)

    # Blue circle (40%)
    with doc.transparent(0.4):
        doc.fill(Color(0.0, 0.0, 1.0))
        doc.circle((circle_cx + 20, circle_cy - 35), 35)

    # Overlapping rectangles with transparency
    rect_x = 320.0
    rect_top_y = 155.0

    # Red rect (100%)
    doc.fill(Color(0.85, 0.2, 0.3))
    doc.rect((rect_x, rect_top_y), 80, 55)

    # Blue rect (65%)
    with doc.transparent(0.65):
        doc.fill(Color(0.2, 0.6, 0.9))
        doc.rect((rect_x + 45, rect_top_y), 80, 55)

    # Green rect (35%)
    with doc.transparent(0.35):
        doc.fill(Color(0.3, 0.85, 0.3))
        doc.rect((rect_x + 22, rect_top_y + 30), 80, 55)

    # Labels
    doc.font("Helvetica", 9)
    doc.text_at("Circles: 100%, 70%, 40%", (60, 70))
    doc.text_at("Rectangles: 100%, 65%, 35%", (320, 70))

    page_idx += 1
    png_page = page_idx

    # Embedded PNG

    doc.new_page()

    margin = 36.0
    header_height = 32.0

    doc.font("Helvetica", 14)
    doc.text_at("Embedded PNG", (margin, PAGE_HEIGHT - margin - 16))

    if os.path.exists("examples/example.png"):
        # Calculate fit dimensions (matches Rust fit_image function)
        available_width = PAGE_WIDTH - 2 * margin
        available_height = PAGE_HEIGHT - 2 * margin - header_height

        # Position is bottom-left corner of available area, image will be centered within fit bounds
        doc.image_at("examples/example.png", (margin, margin), fit=(available_width, available_height))
    else:
        doc.font("Helvetica-Oblique", 11)
        doc.text_at("(example.png not found - run from project root)", (margin, 700))

    page_idx += 1
    jpeg_page = page_idx

    # Embedded JPEG

    doc.new_page()

    doc.font("Helvetica", 14)
    doc.text_at("JPEG (converted from PNG at runtime)", (margin, PAGE_HEIGHT - margin - 16))

    if os.path.exists("examples/example.png"):
        # Convert PNG to JPEG at runtime
        try:
            from PIL import Image
            img = Image.open("examples/example.png")
            rgb_img = img.convert("RGB")
            jpeg_buffer = io.BytesIO()
            rgb_img.save(jpeg_buffer, format="JPEG", quality=85)
            jpeg_bytes = jpeg_buffer.getvalue()

            available_width = PAGE_WIDTH - 2 * margin
            available_height = PAGE_HEIGHT - 2 * margin - header_height

            # Position is bottom-left corner of available area, image will be centered within fit bounds
            doc.image_bytes(jpeg_bytes, "jpeg", (margin, margin), fit=(available_width, available_height))
        except ImportError:
            doc.font("Helvetica-Oblique", 11)
            doc.text_at("(PIL/Pillow not installed - needed for PNG to JPEG conversion)", (margin, 700))
    else:
        doc.font("Helvetica-Oblique", 11)
        doc.text_at("(example.png not found - run from project root)", (margin, 700))

    page_idx += 1
    alpha_page = page_idx

    # PNG with Alpha Transparency

    doc.new_page()

    doc.font("Helvetica", 14)
    doc.text_at("PNG with alpha transparency", (margin, PAGE_HEIGHT - margin - 16))

    if os.path.exists("examples/example.png"):
        available_width = PAGE_WIDTH - 2 * margin
        available_height = PAGE_HEIGHT - 2 * margin - header_height

        # Position is bottom-left corner of available area, image will be centered within fit bounds
        doc.image_at("examples/example.png", (margin, margin), fit=(available_width, available_height))
    else:
        doc.font("Helvetica-Oblique", 11)
        doc.text_at("(example-alpha.png not found - run from project root)", (margin, 700))

    page_idx += 1
    custom_font_page = page_idx

    # Custom Font Embedding

    doc.new_page()

    font_margin = 48.0

    # Header band
    doc.fill(Color.gray(0.95))
    doc.rect((0, PAGE_HEIGHT), PAGE_WIDTH, 82)

    FONT_PATH = "examples/fonts/Roboto-Regular.ttf"
    FONT_BOLD_PATH = "examples/fonts/Roboto-Bold.ttf"
    FONT_ITALIC_PATH = "examples/fonts/Roboto-Italic.ttf"

    if os.path.exists(FONT_PATH) and os.path.exists(FONT_BOLD_PATH) and os.path.exists(FONT_ITALIC_PATH):
        # Embed fonts
        font_regular = doc.embed_font(FONT_PATH)
        font_bold = doc.embed_font(FONT_BOLD_PATH)
        font_italic = doc.embed_font(FONT_ITALIC_PATH)

        # Page title (using embedded font)
        doc.font(font_bold.name, 28)
        doc.text_at("Custom Font Embedding", (font_margin, 800))

        doc.font(font_regular.name, 12)
        doc.text_at("TrueType fonts with full Unicode support", (font_margin, 778))

        # Section 1: Font showcase
        y = 700.0

        doc.font("Helvetica", 14)
        doc.text_at("Font Comparison:", (font_margin, y))
        y -= 30.0

        # Standard font
        doc.font("Helvetica", 16)
        doc.text_at("Helvetica (Standard): The quick brown fox jumps over the lazy dog.", (font_margin, y))
        y -= 25.0

        # Embedded fonts
        doc.font(font_regular.name, 16)
        doc.text_at("Roboto Regular: The quick brown fox jumps over the lazy dog.", (font_margin, y))
        y -= 25.0

        doc.font(font_bold.name, 16)
        doc.text_at("Roboto Bold: The quick brown fox jumps over the lazy dog.", (font_margin, y))
        y -= 25.0

        doc.font(font_italic.name, 16)
        doc.text_at("Roboto Italic: The quick brown fox jumps over the lazy dog.", (font_margin, y))
        y -= 45.0

        # Section 2: Size variations
        doc.font("Helvetica", 14)
        doc.text_at("Size Variations:", (font_margin, y))
        y -= 25.0

        for size in [10.0, 14.0, 18.0, 24.0, 32.0]:
            doc.font(font_regular.name, size)
            doc.text_at(f"{int(size)}pt: Roboto Font", (font_margin, y))
            y -= size + 8.0
        y -= 20.0

        # Section 3: Text measurement
        doc.font("Helvetica", 14)
        doc.text_at("Text Measurement:", (font_margin, y))
        y -= 30.0

        doc.font(font_regular.name, 18)
        sample_text = "Measured Text Width"
        text_width = doc.measure_text(sample_text)

        # Draw the text
        doc.text_at(sample_text, (font_margin, y))

        # Draw a line under it showing the measured width
        doc.stroke(Color(0.9, 0.2, 0.2), 2.0)
        doc.line((font_margin, y - 5), (font_margin + text_width, y - 5))

        doc.font(font_regular.name, 11)
        doc.text_at(f"Width: {text_width:.1f} points at 18pt", (font_margin, y - 20))
        y -= 60.0

        # Section 4: Mixed content
        doc.font("Helvetica", 14)
        doc.text_at("Mixed Fonts in Document:", (font_margin, y))
        y -= 30.0

        # Draw a box with mixed font content
        doc.stroke(Color.gray(0.7), 1.0)
        doc.rounded_rect((font_margin, y + 10), PAGE_WIDTH - font_margin * 2, 90, 8, do_fill=False, do_stroke=True)

        doc.font(font_bold.name, 14)
        doc.text_at("Note:", (font_margin + 15, y - 10))

        doc.font(font_regular.name, 12)
        doc.text_at("This PDF demonstrates seamless mixing of standard PDF fonts", (font_margin + 15, y - 30))
        doc.text_at("(Helvetica, Times, Courier) with embedded TrueType fonts (Roboto).", (font_margin + 15, y - 45))
        doc.text_at("Text is fully searchable and can be copied from the PDF.", (font_margin + 15, y - 60))
    else:
        doc.font("Helvetica", 24)
        doc.text_at("Custom Font Embedding", (font_margin, 800))

        doc.font("Helvetica-Oblique", 11)
        doc.text_at("(Roboto fonts not found - download from Google Fonts)", (font_margin, 700))

    page_idx += 1

    # Ligatures and Kerning

    doc.new_page()

    lig_margin = 48.0

    # Header band
    doc.fill(Color.gray(0.95))
    doc.rect((0, PAGE_HEIGHT), PAGE_WIDTH, 82)

    MAPLE_FONT_PATH = "examples/fonts/MapleMono-NF-CN-Regular.ttf"
    ROBOTO_FONT_PATH = "examples/fonts/Roboto-Regular.ttf"

    if os.path.exists(MAPLE_FONT_PATH) and os.path.exists(ROBOTO_FONT_PATH):
        maple_font = doc.embed_font(MAPLE_FONT_PATH)
        roboto_font = doc.embed_font(ROBOTO_FONT_PATH)

        doc.font(maple_font.name, 28)
        doc.text_at("MapleMono Ligatures", (lig_margin, 800))

        doc.font("Helvetica", 12)
        doc.text_at("Ligatures, kerning, and line spacing", (lig_margin, 778))

        y = 700.0
        doc.font(maple_font.name, 22)

        # Ligature samples
        samples = ["== != === !== <= >= -> => <-> <=>"]
        for line in samples:
            doc.text_at(line, (lig_margin, y))
            y -= 32.0

        y -= 8.0
        doc.font("Helvetica", 11)
        doc.text_at("Nerd Font glyphs (MapleMono NF):", (lig_margin, y))
        y -= 32.0

        doc.font(maple_font.name, 24)
        doc.text_at("\uf09b  \uf121  \uf179  \uf0f3  \uf0e0  \uf2db  \uf1eb", (lig_margin, y))
        y -= 36.0

        doc.stroke(Color.gray(0.88), 0.5)
        doc.line((lig_margin, y), (PAGE_WIDTH - lig_margin, y))
        y -= 16.0

        doc.font("Helvetica", 12)
        doc.text_at("Kerning samples (Roboto, proportional):", (lig_margin, y))
        y -= 18.0

        doc.font("Helvetica", 10)
        doc.text_at("Kerning OFF:", (lig_margin, y))
        y -= 26.0

        doc.font(roboto_font.name, 32)
        doc.text_at_no_kerning("AV AVA WA We To Ta Te Yo", (lig_margin, y))
        y -= 48.0

        doc.font("Helvetica", 10)
        doc.text_at("Kerning ON:", (lig_margin, y))
        y -= 26.0

        doc.font(roboto_font.name, 32)
        doc.text_at("AV AVA WA We To Ta Te Yo", (lig_margin, y))
        y -= 48.0

        doc.font("Helvetica", 12)
        doc.text_at("Line spacing (manual):", (lig_margin, y))
        y -= 18.0

        for spacing in [16.0, 24.0, 36.0]:
            doc.font("Helvetica", 10)
            doc.text_at(f"Line height {spacing:.0f}pt", (lig_margin, y))
            text_y = y - 14.0

            doc.stroke(Color.gray(0.8), 0.5)
            doc.line((lig_margin, text_y), (PAGE_WIDTH - lig_margin, text_y))
            doc.line((lig_margin, text_y - spacing), (PAGE_WIDTH - lig_margin, text_y - spacing))

            doc.font(maple_font.name, 14)
            doc.text_at("The quick brown fox jumps.", (lig_margin, text_y))
            doc.text_at("Second line for spacing.", (lig_margin, text_y - spacing))

            y = text_y - spacing - 24.0
    else:
        doc.font("Helvetica", 24)
        doc.text_at("MapleMono Ligatures", (lig_margin, 800))

        doc.font("Helvetica-Oblique", 11)
        doc.text_at("(MapleMono NF font not found)", (lig_margin, 700))

    page_idx += 1

    # CJK Font Support

    doc.new_page()

    cjk_margin = 48.0

    # Header band
    doc.fill(Color.gray(0.95))
    doc.rect((0, PAGE_HEIGHT), PAGE_WIDTH, 82)

    CJK_FONT_LIGHT = "examples/fonts/LXGWWenKai-Light.ttf"
    CJK_FONT_REGULAR = "examples/fonts/LXGWWenKai-Regular.ttf"
    CJK_FONT_MEDIUM = "examples/fonts/LXGWWenKai-Medium.ttf"

    if os.path.exists(CJK_FONT_LIGHT) and os.path.exists(CJK_FONT_REGULAR) and os.path.exists(CJK_FONT_MEDIUM):
        cjk_light = doc.embed_font(CJK_FONT_LIGHT)
        cjk_regular = doc.embed_font(CJK_FONT_REGULAR)
        cjk_medium = doc.embed_font(CJK_FONT_MEDIUM)

        # Page title
        doc.font(cjk_medium.name, 28)
        doc.text_at("CJK 字体支持", (cjk_margin, 800))

        doc.font(cjk_regular.name, 12)
        doc.text_at("中日韩文字渲染 / Chinese, Japanese, Korean", (cjk_margin, 778))

        y = 710.0

        # Section 1: Chinese text samples
        doc.font("Helvetica", 14)
        doc.text_at("Chinese Text Samples:", (cjk_margin, y))
        y -= 30.0

        doc.font(cjk_light.name, 16)
        doc.text_at("Light: 霞鹜文楷是一款开源中文字体", (cjk_margin, y))
        y -= 25.0

        doc.font(cjk_regular.name, 16)
        doc.text_at("Regular: 天地玄黄，宇宙洪荒", (cjk_margin, y))
        y -= 25.0

        doc.font(cjk_medium.name, 16)
        doc.text_at("Medium: 日月盈昃，辰宿列张", (cjk_margin, y))
        y -= 40.0

        # Section 2: Mixed content
        doc.font("Helvetica", 14)
        doc.text_at("Mixed Language Content:", (cjk_margin, y))
        y -= 30.0

        doc.font(cjk_regular.name, 14)
        doc.text_at("PDF 库 pdfcrate 支持 TrueType 字体嵌入", (cjk_margin, y))
        y -= 22.0
        doc.text_at("支持字体子集化 (Font Subsetting) 以减小文件大小", (cjk_margin, y))
        y -= 22.0
        doc.text_at("可用于 Cloudflare Workers 等 WASM 环境", (cjk_margin, y))
        y -= 40.0

        # Section 3: Classical Chinese
        doc.font("Helvetica", 14)
        doc.text_at("Classical Chinese:", (cjk_margin, y))
        y -= 30.0

        doc.font(cjk_regular.name, 13)
        classical_lines = [
            "子曰：「学而时习之，不亦说乎？",
            "有朋自远方来，不亦乐乎？",
            "人不知而不愠，不亦君子乎？」",
            "",
            "《论语·学而》",
        ]
        for line in classical_lines:
            doc.text_at(line, (cjk_margin, y))
            y -= 20.0
        y -= 20.0

        # Section 4: Japanese text
        doc.font("Helvetica", 14)
        doc.text_at("Japanese Text:", (cjk_margin, y))
        y -= 30.0

        doc.font(cjk_regular.name, 14)
        doc.text_at("ひらがな: あいうえお かきくけこ", (cjk_margin, y))
        y -= 22.0
        doc.text_at("カタカナ: アイウエオ カキクケコ", (cjk_margin, y))
        y -= 22.0
        doc.text_at("漢字混じり: 東京は日本の首都です", (cjk_margin, y))
        y -= 40.0

        # Section 5: Size variations with CJK
        doc.font("Helvetica", 14)
        doc.text_at("Size Variations:", (cjk_margin, y))
        y -= 25.0

        for size in [10.0, 14.0, 18.0, 24.0]:
            doc.font(cjk_regular.name, size)
            doc.text_at(f"{int(size)}pt: 中文字体大小测试", (cjk_margin, y))
            y -= size + 8.0

        # Footer note
        doc.font("Helvetica", 10)
        doc.text_at("Note: CJK fonts are subsetted - only used glyphs are embedded to reduce file size.", (cjk_margin, 60))
    else:
        doc.font("Helvetica", 24)
        doc.text_at("CJK Font Support", (cjk_margin, 800))

        doc.font("Helvetica-Oblique", 11)
        doc.text_at("(LXGW WenKai fonts not found)", (cjk_margin, 700))

    page_idx += 1
    forms_page = page_idx

    # Interactive Forms

    doc.new_page()

    form_margin = 48.0

    # Header band
    doc.fill(Color.gray(0.95))
    doc.rect((0, PAGE_HEIGHT), PAGE_WIDTH, 82)

    doc.font("Helvetica", 24)
    doc.text_at("Interactive Forms", (form_margin, 800))

    doc.font("Helvetica", 11)
    doc.text_at("AcroForms - text fields, checkboxes, and dropdowns", (form_margin, 780))

    y = 700.0
    label_x = form_margin
    field_x = form_margin + 120
    field_width = 200.0
    field_height = 20.0
    row_height = 35.0

    # Section: Contact Information
    doc.font("Helvetica", 14)
    doc.text_at("Contact Information", (label_x, y))
    y -= row_height

    doc.font("Helvetica", 11)

    # Name field
    doc.text_at("Name:", (label_x, y + 5))
    doc.text_field("name", (field_x, y, field_x + field_width, y + field_height))
    y -= row_height

    # Email field
    doc.text_at("Email:", (label_x, y + 5))
    doc.text_field("email", (field_x, y, field_x + field_width, y + field_height))
    y -= row_height

    # Phone field
    doc.text_at("Phone:", (label_x, y + 5))
    doc.text_field("phone", (field_x, y, field_x + 150, y + field_height))
    y -= row_height + 20

    # Section: Preferences
    doc.font("Helvetica", 14)
    doc.text_at("Preferences", (label_x, y))
    y -= row_height

    doc.font("Helvetica", 11)

    # Newsletter checkbox
    doc.text_at("Subscribe:", (label_x, y + 5))
    doc.checkbox("newsletter", (field_x, y, field_x + 18, y + 18), checked=True)
    doc.text_at("Newsletter", (field_x + 25, y + 5))
    y -= row_height

    # Updates checkbox
    doc.text_at("Receive:", (label_x, y + 5))
    doc.checkbox("updates", (field_x, y, field_x + 18, y + 18), checked=False)
    doc.text_at("Product updates", (field_x + 25, y + 5))
    y -= row_height + 20

    # Section: Selection
    doc.font("Helvetica", 14)
    doc.text_at("Selection", (label_x, y))
    y -= row_height

    doc.font("Helvetica", 11)

    # Country dropdown
    doc.text_at("Country:", (label_x, y + 5))
    doc.dropdown("country", (field_x, y, field_x + field_width, y + field_height),
                 ["USA", "Canada", "UK", "Germany", "France", "Japan"])
    y -= row_height

    # Department dropdown
    doc.text_at("Department:", (label_x, y + 5))
    doc.dropdown("department", (field_x, y, field_x + 150, y + field_height),
                 ["Sales", "Engineering", "Marketing", "Support"])

    # Footer note
    doc.font("Helvetica", 10)
    doc.text_at("Note: Form fields are interactive - click to edit in a PDF viewer.", (form_margin, 80))

    page_idx += 1

    # SVG Barcode

    doc.new_page()

    svg_margin = 48.0

    # Header band
    doc.fill(Color.gray(0.95))
    doc.rect((0, PAGE_HEIGHT), PAGE_WIDTH, 82)

    doc.font("Helvetica", 24)
    doc.text_at("SVG Barcode", (svg_margin, 800))

    doc.font("Helvetica", 11)
    doc.text_at("SVG rendering", (svg_margin, 780))

    doc.font("Helvetica", 12)
    doc.text_at("The barcode below is drawn from SVG (basic shapes are converted to paths).", (svg_margin, 720))

    barcode_svg = '''
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
'''

    target_width = PAGE_WIDTH - svg_margin * 2
    target_height = 140.0
    x = svg_margin
    y = 520.0

    # Background rectangle (using top-left origin)
    rect_top_y = y + target_height + 8
    doc.fill(Color.gray(0.97))
    doc.rect((x - 8, rect_top_y), target_width + 16, target_height + 16)
    doc.stroke(Color.gray(0.85), 0.5)
    doc.rect((x - 8, rect_top_y), target_width + 16, target_height + 16, do_fill=False, do_stroke=True)

    doc.draw_svg(barcode_svg, (x, y), target_width, target_height)

    doc.font("Helvetica", 10)
    doc.text_at("Use SVG for barcodes, charts, and icons without rasterization.", (svg_margin, 470))

    page_idx += 1
    layout_page = page_idx

    # LayoutDocument Demo

    doc.new_page()

    # Header band
    doc.fill(Color.gray(0.95))
    doc.rect((0, PAGE_HEIGHT), PAGE_WIDTH, 82)

    doc.font("Helvetica", 24)
    doc.text_at("LayoutDocument Demo", (36, 800))

    doc.font("Helvetica", 11)
    doc.text_at("Prawn-style cursor-based layout (no manual coordinate calculation)", (36, 780))

    # Reset to layout mode with proper margins
    # Match Rust: top_margin = 136 (136pt from page top = 842-136 = 706 cursor start)
    # Python already has 36pt top margin, so move down 136-36 = 100 to reach same position
    doc.move_down(100)

    # Section 1: Bounding Box Demo
    doc.font("Helvetica", 12)
    doc.text("1. Nested Bounding Boxes:")
    doc.move_down(10)

    # Outer box (fixed height)
    # Prawn-style: y is position from bounds.bottom, pass cursor() to place at current position
    y = doc.cursor()
    with doc.bounding_box(220, height=90, y=y):
        doc.stroke_bounds()
        doc.font("Helvetica", 10)
        doc.text("Outer box (220x90)")
        doc.move_down(5)

        # Inner box (nested)
        inner_y = doc.cursor()
        with doc.bounding_box(160, x=15, y=inner_y):
            doc.stroke_bounds()
            doc.font("Helvetica", 9)
            doc.text("Inner nested box")
            doc.text("Auto-height (stretchy)")

    # Side-by-side boxes
    doc.move_down(15)
    doc.font("Helvetica", 12)
    doc.text("2. Side-by-Side Layout:")
    doc.move_down(10)

    box_top = doc.cursor()

    # Left box
    with doc.bounding_box(140, height=60, y=box_top):
        doc.stroke_bounds()
        doc.font("Helvetica", 9)
        doc.text("Left Box")
        doc.text("Width: 140pt")

    # Right box (same y level)
    doc.set_cursor(box_top)
    with doc.bounding_box(140, height=60, x=160, y=box_top):
        doc.stroke_bounds()
        doc.font("Helvetica", 9)
        doc.text("Right Box")
        doc.text("Offset: 160pt")

    doc.move_down(15)

    # Section 2: Formatted text (mixed styles)
    doc.font("Helvetica", 12)
    doc.text("3. Formatted Text (Mixed Styles):")
    doc.move_down(8)

    doc.font("Helvetica", 10)
    doc.formatted_text([
        TextFragment("This is "),
        TextFragment("bold", bold=True),
        TextFragment(", "),
        TextFragment("italic", italic=True),
        TextFragment(", and "),
        TextFragment("red", color=Color(1.0, 0.0, 0.0)),
        TextFragment(" text in one line."),
    ])
    doc.formatted_text([
        TextFragment("Mixed: ", bold=True),
        TextFragment("Times ", font="Times-Roman"),
        TextFragment("and "),
        TextFragment("Courier", font="Courier"),
        TextFragment(" fonts."),
    ])

    doc.move_down(10)

    # Section 3: Cursor tracking
    doc.font("Helvetica", 12)
    doc.text("4. Cursor Tracking:")
    doc.move_down(8)

    doc.font("Helvetica", 10)
    cursor1 = doc.cursor()
    doc.text(f"Cursor at: {cursor1:.1f}pt")
    doc.text("Each text() call auto-advances cursor")
    cursor2 = doc.cursor()
    doc.text(f"Now at: {cursor2:.1f}pt (moved {cursor1 - cursor2:.1f}pt)")

    doc.move_down(15)

    # Section 4: Indent
    doc.font("Helvetica", 12)
    doc.text("5. Indent:")
    doc.move_down(8)

    doc.font("Helvetica", 10)
    doc.text("Normal margin.")
    with doc.indent(30):
        doc.text("Indented 30pt from left.")
        with doc.indent(30):
            doc.text("Double indent (60pt total).")
        doc.text("Back to 30pt indent.")
    doc.text("Back to normal.")

    doc.move_down(15)

    # Section 5: Float
    doc.font("Helvetica", 12)
    doc.text("6. Float (temp position):")
    doc.move_down(8)

    doc.font("Helvetica", 10)
    doc.text("Before float")
    with doc.float():
        doc.move_down(40)
        doc.font("Helvetica", 9)
        doc.text(">> Floated 40pt down")
    doc.text("After float (continues from 'Before')")
    doc.move_down(50)

    # Section 6: Bounds visualization
    doc.font("Helvetica", 12)
    doc.text("7. Bounds Visualization:")
    doc.move_down(8)

    y = doc.cursor()
    with doc.bounding_box(200, height=50, y=y):
        doc.stroke_bounds()
        doc.font("Helvetica", 9)
        doc.text("stroke_bounds() draws")
        doc.text("the current bounding box")

    page_idx += 1

    # Text Box Overflow Modes

    doc.new_page()

    # Header band
    doc.fill(Color.gray(0.95))
    doc.rect((0, PAGE_HEIGHT), PAGE_WIDTH, 82)

    doc.font("Helvetica", 24)
    doc.text_at("Text Box Overflow Modes", (48, 804))

    doc.font("Helvetica", 11)
    doc.text_at("Truncate, ShrinkToFit, and Expand behaviors", (48, 784))

    # Sample text that will overflow
    long_text = ("Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
                 "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. "
                 "Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. "
                 "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum.")

    # Reset cursor for layout mode
    # Match Rust: top_margin = 136, left margin = 48
    doc.move_down(100)  # 136 - 36 = 100 (account for existing margin)

    # Indent to match Rust's 48pt left margin (our margin is 36pt, so indent 12pt)
    doc.indent_push(12, 0)

    box_width = 220.0
    box_height = 50.0
    padding = 4.0

    # Section 1: Overflow::Truncate
    doc.font("Helvetica-Bold", 14)
    doc.text("1. Overflow::Truncate (default)")
    doc.move_down(8)

    doc.font("Helvetica", 9)
    doc.text("Text that exceeds the box height is silently discarded:")
    doc.move_down(10)

    # Draw border and text in the same bounding_box
    y = doc.cursor()
    outer_height = box_height + padding * 2
    with doc.bounding_box(box_width + padding * 2, height=outer_height, y=y):
        doc.stroke_bounds()
        doc.font("Helvetica", 9)
        # Prawn-style: point[1] is Y from bounds.bottom, place at top with padding
        truncate_result = doc.text_box(long_text, (padding, outer_height - padding), box_width, box_height, Overflow.Truncate)

    doc.move_down(10)
    doc.font("Helvetica", 8)
    doc.text(f"Result: truncated={truncate_result.truncated}, lines_rendered={truncate_result.lines_rendered}, total_lines={truncate_result.total_lines}")

    doc.move_down(25)

    # Section 2: Overflow::ShrinkToFit
    doc.font("Helvetica-Bold", 14)
    doc.text("2. Overflow::ShrinkToFit(min_size)")
    doc.move_down(8)

    doc.font("Helvetica", 9)
    doc.text("Font size is reduced until text fits (minimum 6pt):")
    doc.move_down(10)

    y = doc.cursor()
    outer_height = box_height + padding * 2
    with doc.bounding_box(box_width + padding * 2, height=outer_height, y=y):
        doc.stroke_bounds()
        doc.font("Helvetica", 9)
        shrink_result = doc.text_box(long_text, (padding, outer_height - padding), box_width, box_height, shrink_to_fit=6.0)

    doc.move_down(10)
    doc.font("Helvetica", 8)
    doc.text(f"Result: font_size={shrink_result.font_size:.1f}pt (was 9pt), truncated={shrink_result.truncated}, lines={shrink_result.lines_rendered}")

    doc.move_down(25)

    # Section 3: Overflow::Expand
    doc.font("Helvetica-Bold", 14)
    doc.text("3. Overflow::Expand")
    doc.move_down(8)

    doc.font("Helvetica", 9)
    doc.text("Box height expands to fit all content:")
    doc.move_down(10)

    # For Expand, render text first, then draw border with actual height
    cursor_before = doc.cursor()
    doc.font("Helvetica", 9)
    # Prawn-style: point[1] is Y from bounds.bottom
    # Subtract padding from y to create top padding (text starts below border top)
    expand_result = doc.text_box(long_text, (padding, cursor_before - padding), box_width, box_height, Overflow.Expand)

    # Draw border around the expanded content using float
    with doc.float():
        doc.set_cursor(cursor_before)
        with doc.bounding_box(box_width + padding * 2, height=expand_result.height + padding * 2, y=cursor_before):
            doc.stroke_bounds()

    doc.move_down(10)
    doc.font("Helvetica", 8)
    doc.text(f"Result: actual_height={expand_result.height:.1f}pt (min {box_height}pt), lines={expand_result.lines_rendered}")

    doc.move_down(25)

    # Section 4: Comparison - same text, all three modes side by side
    doc.font("Helvetica-Bold", 14)
    doc.text("4. Side-by-Side Comparison")
    doc.move_down(8)

    doc.font("Helvetica", 9)
    doc.text("Same text in 150x45pt boxes:")
    doc.move_down(10)

    compare_width = 150.0
    compare_height = 45.0
    gap = 15.0
    small_padding = 2.0
    compare_text = "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. How vexingly quick daft zebras jump!"

    row_top = doc.cursor()

    # Box 1: Truncate
    with doc.bounding_box(compare_width, height=compare_height, y=row_top):
        doc.stroke_bounds()
        doc.font("Helvetica", 9)
        # Prawn-style: point[1] is Y from bounds.bottom
        doc.text_box(compare_text, (small_padding, compare_height - small_padding),
                     compare_width - small_padding * 2, compare_height - small_padding * 2,
                     Overflow.Truncate)

    # Box 2: ShrinkToFit
    doc.set_cursor(row_top)
    with doc.bounding_box(compare_width, height=compare_height, x=compare_width + gap, y=row_top):
        doc.stroke_bounds()
        doc.font("Helvetica", 9)
        doc.text_box(compare_text, (small_padding, compare_height - small_padding),
                     compare_width - small_padding * 2, compare_height - small_padding * 2,
                     shrink_to_fit=5.0)

    # Box 3: Expand
    doc.set_cursor(row_top)
    doc.font("Helvetica", 9)
    # Prawn-style: point[1] is Y from bounds.bottom
    # Subtract small_padding from y to create top padding
    expand_cmp_result = doc.text_box(compare_text, ((compare_width + gap) * 2 + small_padding, row_top - small_padding),
                                      compare_width - small_padding * 2, compare_height - small_padding * 2,
                                      Overflow.Expand)

    # Draw border for expanded box
    with doc.float():
        doc.set_cursor(row_top)
        with doc.bounding_box(compare_width, height=expand_cmp_result.height + small_padding * 2, x=(compare_width + gap) * 2, y=row_top):
            doc.stroke_bounds()

    # Labels below the boxes
    max_box_height = max(compare_height, expand_cmp_result.height + small_padding * 2)
    doc.set_cursor(row_top - max_box_height - 5)

    # Convert relative cursor to absolute Y for text_at (same as Rust version)
    left_margin = 48.0
    label_y = doc.bounds_bottom() + doc.cursor()
    doc.font("Helvetica", 7)
    doc.text_at("Truncate", (left_margin + small_padding, label_y))
    doc.text_at("ShrinkToFit(5.0)", (left_margin + compare_width + gap + small_padding, label_y))
    doc.text_at(f"Expand (h={expand_cmp_result.height + small_padding * 2:.0f})", (left_margin + (compare_width + gap) * 2 + small_padding, label_y))

    # Pop the indent we added at the start of this page
    doc.indent_pop(12, 0)

    page_idx += 1
    layout_advanced_page = page_idx

    # Text Layout Features

    doc.new_page()

    # Header band
    doc.fill(Color.gray(0.95))
    doc.rect((0, PAGE_HEIGHT), PAGE_WIDTH, 82)

    doc.font("Helvetica", 24)
    doc.text_at("Text Layout Features", (48, 804))

    doc.font("Helvetica", 11)
    doc.text_at("Text alignment, leading, wrapping & text boxes", (48, 784))

    # Use absolute positioning for content (matches Rust)
    left_margin = 48.0
    y = 720.0  # Start below header

    # Section 1: Text Alignment
    doc.font("Helvetica-Bold", 12)
    doc.text_at("1. Text Alignment", (left_margin, y))
    y -= 18

    doc.font("Helvetica", 9)
    doc.text_at("Left aligned text (default)", (left_margin, y))
    y -= 12

    # Center (approximate)
    doc.text_at("Center aligned text", ((PAGE_WIDTH - 120) / 2, y))
    y -= 12

    # Right aligned
    doc.text_at("Right aligned text", (PAGE_WIDTH - left_margin - 100, y))
    y -= 20

    # Section 2: Leading
    doc.font("Helvetica-Bold", 12)
    doc.text_at("2. Leading (Line Spacing)", (left_margin, y))
    y -= 16

    # Left column
    col1_x = left_margin
    col2_x = 320.0
    leading_y = y

    doc.font("Helvetica", 8.5)
    doc.text_at("Default leading (1.2x):", (col1_x, y))
    y -= 10
    doc.font("Helvetica", 8)
    doc.text_at("  Line 1 with normal spacing", (col1_x, y))
    y -= 10
    doc.text_at("  Line 2 with normal spacing", (col1_x, y))
    y -= 14

    doc.font("Helvetica", 8.5)
    doc.text_at("Tight leading (1.0x):", (col1_x, y))
    y -= 9
    doc.font("Helvetica", 8)
    doc.text_at("  Line 1 with tight spacing", (col1_x, y))
    y -= 9
    doc.text_at("  Line 2 with tight spacing", (col1_x, y))

    # Right column - Loose
    y2 = leading_y
    doc.font("Helvetica", 8.5)
    doc.text_at("Loose leading (1.8x):", (col2_x, y2))
    y2 -= 14
    doc.font("Helvetica", 8)
    doc.text_at("  Line 1 with loose spacing", (col2_x, y2))
    y2 -= 14
    doc.text_at("  Line 2 with loose spacing", (col2_x, y2))

    y -= 22

    # Section 3: Text Wrapping
    doc.font("Helvetica-Bold", 12)
    doc.text_at("3. Automatic Text Wrapping", (left_margin, y))
    y -= 14

    doc.font("Helvetica", 8.5)
    wrap_lines = [
        "This demonstrates automatic text wrapping. The text automatically wraps to fit within the",
        "available width, making it easy to create flowing paragraphs without manual line breaks.",
    ]
    for line in wrap_lines:
        doc.text_at(line, (left_margin, y))
        y -= 10
    y -= 10

    # Section 4: Text Box
    doc.font("Helvetica-Bold", 12)
    doc.text_at("4. Text Box (Fixed Height)", (left_margin, y))
    y -= 14

    # Draw two boxes with borders
    box_width = 235.0
    box_height = 55.0
    box1_x = left_margin
    box2_x = left_margin + 265

    # Draw box borders
    doc.stroke(Color.gray(0.6), 0.5)
    doc.rect((box1_x, y), box_width, box_height, do_fill=False, do_stroke=True)
    doc.rect((box2_x, y), box_width, box_height, do_fill=False, do_stroke=True)

    # Text inside boxes
    doc.font("Helvetica", 7.5)
    box1_lines = [
        "Text boxes constrain content to a fixed height.",
        "Overflow is clipped. Useful for predictable layouts",
        "where text must fit within specific boundaries.",
    ]
    box2_lines = [
        "Second text box at the same vertical position.",
        "Each box can have different content while",
        "maintaining consistent structure.",
    ]

    by = y - 5.4
    for line in box1_lines:
        doc.text_at(line, (box1_x + 4, by))
        by -= 8.67

    by = y - 5.4
    for line in box2_lines:
        doc.text_at(line, (box2_x + 4, by))
        by -= 8.67

    y = y - box_height - 25

    # Footer
    doc.font("Helvetica-Oblique", 8)
    doc.text_at("All text layout features work seamlessly with LayoutDocument", (130, y))

    page_idx += 1
    table_page = page_idx

    # Table Demo

    doc.new_page()

    # Header band
    doc.fill(Color.gray(0.95))
    doc.rect((0, PAGE_HEIGHT), PAGE_WIDTH, 82)

    doc.font("Helvetica", 24)
    doc.text_at("Table Demo", (48, 804))

    doc.font("Helvetica", 11)
    doc.text_at("Data tables with headers, row colors, and styling", (48, 784))

    # Reset for layout mode
    doc.move_down(100)

    # Basic table
    doc.font("Helvetica-Bold", 14)
    doc.text("1. Basic Table")
    doc.move_down(10)

    doc.font("Helvetica", 10)
    doc.table([
        ["Name", "Age", "City"],
        ["Alice", "30", "New York"],
        ["Bob", "25", "Los Angeles"],
        ["Charlie", "35", "Chicago"],
    ])
    doc.move_down(20)

    # Table with fixed column widths
    doc.font("Helvetica-Bold", 14)
    doc.text("2. Fixed Column Widths")
    doc.move_down(10)

    doc.font("Helvetica", 10)
    doc.table([
        ["Product", "Quantity", "Price", "Total"],
        ["Widget A", "10", "$5.00", "$50.00"],
        ["Widget B", "5", "$12.00", "$60.00"],
        ["Widget C", "20", "$2.50", "$50.00"],
    ], TableOptions(column_widths=[150, 80, 80, 80]))
    doc.move_down(20)

    # Table with header and row colors
    doc.font("Helvetica-Bold", 14)
    doc.text("3. Table with Row Colors")
    doc.move_down(10)

    doc.font("Helvetica", 10)
    doc.table([
        ["ID", "Status", "Description"],
        ["001", "Active", "First item"],
        ["002", "Pending", "Second item"],
        ["003", "Active", "Third item"],
        ["004", "Inactive", "Fourth item"],
    ], TableOptions(header=1, row_colors=[Color.white(), Color.gray(0.95)]))

    page_idx += 1
    grid_page = page_idx

    # Grid System

    doc.new_page()

    # Define a 6-row, 4-column grid with 10pt gutters
    doc.define_grid(rows=6, columns=4, gutter=10)

    # Row 0: Header cells with title
    with doc.grid_span((0, 0), (0, 3)):
        doc.stroke_bounds()
        doc.font("Helvetica-Bold", 24)
        doc.text("Grid System")
        doc.move_down(5)
        doc.font("Helvetica", 11)
        doc.text("Prawn-style grid layout for precise positioning")

    # Row 1: Vertical span and large span
    with doc.grid_span((1, 0), (2, 0)):
        doc.stroke_bounds()
        doc.font("Helvetica-Bold", 10)
        doc.text("Vertical")
        doc.font("Helvetica", 9)
        doc.text("(1,0)-(2,0)")

    with doc.grid_span((1, 1), (2, 3)):
        doc.stroke_bounds()
        doc.font("Helvetica-Bold", 12)
        doc.text("Large Span (1,1)-(2,3)")
        doc.move_down(5)
        doc.font("Helvetica", 10)
        doc.text("This span covers 2 rows and 3 columns.")
        doc.text("Perfect for content areas, sidebars, or")
        doc.text("any layout requiring multiple cells.")

    # Row 3-4: Nav, Main Content, Side
    with doc.grid_cell(3, 0):
        doc.stroke_bounds()
        doc.font("Helvetica", 9)
        doc.text("Nav")

    with doc.grid_span((3, 1), (4, 3)):
        doc.stroke_bounds()
        doc.font("Helvetica-Bold", 11)
        doc.text("Main Content Area")
        doc.move_down(5)
        doc.font("Helvetica", 10)
        doc.text("Grids make it easy to create responsive layouts.")
        doc.text("Define rows, columns, and gutters, then place")
        doc.text("content in cells or spans as needed.")

    with doc.grid_cell(4, 0):
        doc.stroke_bounds()
        doc.font("Helvetica", 9)
        doc.text("Side")

    # Row 5: Footer cells
    for col in range(4):
        with doc.grid_cell(5, col):
            doc.stroke_bounds()
            doc.font("Helvetica", 9)
            doc.text(f"Footer {col + 1}")

    # Build document outline (bookmarks)

    doc.outline([
        OutlineItem("Drawing & Graphics", page=drawing_page, children=[
            OutlineItem("Strokes & Fills", page=drawing_page),
            OutlineItem("Polygons", page=drawing_page),
            OutlineItem("Transparency", page=drawing_page),
        ]),
        OutlineItem("Images", page=png_page, children=[
            OutlineItem("PNG Image", page=png_page),
            OutlineItem("JPEG Image", page=jpeg_page),
            OutlineItem("PNG with Alpha", page=alpha_page),
        ]),
        OutlineItem("Interactive", page=forms_page, children=[
            OutlineItem("AcroForms", page=forms_page),
        ]),
        OutlineItem("Layout System", page=layout_page, children=[
            OutlineItem("LayoutDocument Basics", page=layout_page),
            OutlineItem("Text Layout Features", page=layout_advanced_page),
            OutlineItem("Table Demo", page=table_page),
            OutlineItem("Grid System", page=grid_page),
        ]),
    ])

    # Save the document
    doc.save("showcase_python.pdf")
    print(f"Created: showcase_python.pdf ({page_idx + 1} pages)")


if __name__ == "__main__":
    create_showcase()
