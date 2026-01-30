"""
Tests for pdfcrate rich text (formatted text) features.
"""

import pytest
from pdfcrate import Document, Margin, Color, TextFragment, SpanBuilder


class TestTextFragment:
    """Test TextFragment class."""

    def test_create_simple_fragment(self):
        """Test creating simple text fragment."""
        frag = TextFragment("Hello")
        assert frag is not None

    def test_fragment_with_bold(self):
        """Test creating bold fragment."""
        frag = TextFragment("Bold", bold=True)
        assert frag is not None

    def test_fragment_with_italic(self):
        """Test creating italic fragment."""
        frag = TextFragment("Italic", italic=True)
        assert frag is not None

    def test_fragment_with_color(self):
        """Test creating colored fragment."""
        frag = TextFragment("Red", color=Color.red())
        assert frag is not None

    def test_fragment_with_size(self):
        """Test creating fragment with custom size."""
        frag = TextFragment("Large", size=24)
        assert frag is not None

    def test_fragment_with_font(self):
        """Test creating fragment with custom font."""
        frag = TextFragment("Courier", font="Courier")
        assert frag is not None

    def test_fragment_with_all_options(self):
        """Test creating fragment with all options."""
        frag = TextFragment(
            "Styled",
            bold=True,
            italic=True,
            color=Color.blue(),
            size=18,
            font="Times-Roman"
        )
        assert frag is not None

    def test_fragment_with_underline(self):
        """Test creating fragment with underline."""
        frag = TextFragment("Underlined", underline=True)
        assert frag is not None

    def test_fragment_with_strikethrough(self):
        """Test creating fragment with strikethrough."""
        frag = TextFragment("Struck", strikethrough=True)
        assert frag is not None

    def test_fragment_with_superscript(self):
        """Test creating fragment with superscript."""
        frag = TextFragment("2", superscript=True)
        assert frag is not None

    def test_fragment_with_subscript(self):
        """Test creating fragment with subscript."""
        frag = TextFragment("2", subscript=True)
        assert frag is not None

    def test_fragment_with_link(self):
        """Test creating fragment with link."""
        frag = TextFragment("click", link="https://example.com")
        assert frag is not None

    def test_fragment_with_all_new_options(self):
        """Test creating fragment with all new inline options."""
        frag = TextFragment(
            "Full",
            bold=True,
            italic=True,
            color=Color.red(),
            size=14,
            font="Courier",
            underline=True,
            strikethrough=True,
            superscript=False,
            subscript=False,
            link="https://example.com",
        )
        assert frag is not None


class TestSpanBuilder:
    """Test SpanBuilder fluent API."""

    def test_create_span(self):
        """Test creating a span."""
        span = SpanBuilder("Hello")
        assert span is not None

    def test_span_bold(self):
        """Test bold span."""
        span = SpanBuilder("Bold").bold()
        assert span is not None

    def test_span_italic(self):
        """Test italic span."""
        span = SpanBuilder("Italic").italic()
        assert span is not None

    def test_span_color(self):
        """Test colored span."""
        span = SpanBuilder("Red").color(Color.red())
        assert span is not None

    def test_span_size(self):
        """Test span with size."""
        span = SpanBuilder("Large").size(24)
        assert span is not None

    def test_span_font(self):
        """Test span with font."""
        span = SpanBuilder("Mono").font("Courier")
        assert span is not None

    def test_span_chaining(self):
        """Test chaining multiple span methods."""
        span = SpanBuilder("Styled").bold().italic().color(Color.blue()).size(18)
        assert span is not None

    def test_span_end(self):
        """Test span.end() returns TextFragment."""
        frag = SpanBuilder("Text").bold().end()
        assert isinstance(frag, TextFragment)

    def test_document_span_shortcut(self):
        """Test Document.span() static method."""
        frag = Document.span("Hello").bold().end()
        assert isinstance(frag, TextFragment)

    def test_span_underline(self):
        """Test underline span."""
        span = SpanBuilder("text").underline()
        assert span is not None
        frag = span.end()
        assert isinstance(frag, TextFragment)

    def test_span_strikethrough(self):
        """Test strikethrough span."""
        frag = SpanBuilder("text").strikethrough().end()
        assert isinstance(frag, TextFragment)

    def test_span_superscript(self):
        """Test superscript span."""
        frag = SpanBuilder("2").superscript().end()
        assert isinstance(frag, TextFragment)

    def test_span_subscript(self):
        """Test subscript span."""
        frag = SpanBuilder("2").subscript().end()
        assert isinstance(frag, TextFragment)

    def test_span_link(self):
        """Test link span."""
        frag = SpanBuilder("click").link("https://example.com").end()
        assert isinstance(frag, TextFragment)

    def test_span_all_new_methods_chained(self):
        """Test chaining all new span methods."""
        frag = (
            SpanBuilder("text")
            .bold()
            .underline()
            .strikethrough()
            .link("https://example.com")
            .end()
        )
        assert isinstance(frag, TextFragment)


class TestFormattedText:
    """Test formatted_text method."""

    def test_formatted_text_requires_margin(self):
        """Test that formatted_text requires margin."""
        doc = Document()
        with pytest.raises(RuntimeError):
            doc.formatted_text([TextFragment("Hello")])

    def test_formatted_text_single_fragment(self):
        """Test formatted_text with single fragment."""
        doc = Document(margin=Margin.all(72))
        result = doc.formatted_text([TextFragment("Hello")])
        assert result is doc

    def test_formatted_text_multiple_fragments(self):
        """Test formatted_text with multiple fragments."""
        doc = Document(margin=Margin.all(72))
        result = doc.formatted_text([
            TextFragment("Hello "),
            TextFragment("World", bold=True),
        ])
        assert result is doc

    def test_formatted_text_with_span_builder(self):
        """Test formatted_text with SpanBuilder."""
        doc = Document(margin=Margin.all(72))
        result = doc.formatted_text([
            Document.span("Hello ").end(),
            Document.span("World").bold().end(),
        ])
        assert result is doc

    def test_formatted_text_mixed_styles(self):
        """Test formatted_text with various styles."""
        doc = Document(margin=Margin.all(72))
        result = doc.formatted_text([
            TextFragment("Normal "),
            TextFragment("bold", bold=True),
            TextFragment(", "),
            TextFragment("italic", italic=True),
            TextFragment(", and "),
            TextFragment("red", color=Color.red()),
            TextFragment(" text."),
        ])
        assert result is doc

    def test_formatted_text_with_fonts(self):
        """Test formatted_text with different fonts."""
        doc = Document(margin=Margin.all(72))
        result = doc.formatted_text([
            TextFragment("Helvetica "),
            TextFragment("Times", font="Times-Roman"),
            TextFragment(" and "),
            TextFragment("Courier", font="Courier"),
        ])
        assert result is doc

    def test_formatted_text_with_new_fields(self):
        """Test formatted_text with underline, strikethrough, etc."""
        doc = Document(margin=Margin.all(72))
        result = doc.formatted_text([
            TextFragment("Normal "),
            TextFragment("underlined", underline=True),
            TextFragment(" and "),
            TextFragment("struck", strikethrough=True),
            TextFragment(" text."),
        ])
        assert result is doc


class TestTextInline:
    """Test text_inline method (HTML-like inline formatting)."""

    def test_text_inline_requires_margin(self):
        """Test that text_inline requires margin."""
        doc = Document()
        with pytest.raises(RuntimeError):
            doc.text_inline("Hello <b>world</b>")

    def test_text_inline_plain_text(self):
        """Test text_inline with plain text (no tags)."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("Hello world")
        assert result is doc

    def test_text_inline_bold(self):
        """Test text_inline with bold tag."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("Hello <b>bold</b> world")
        assert result is doc

    def test_text_inline_italic(self):
        """Test text_inline with italic tag."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("Hello <i>italic</i> world")
        assert result is doc

    def test_text_inline_strong_em(self):
        """Test text_inline with strong/em tags."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("<strong>strong</strong> and <em>emphasis</em>")
        assert result is doc

    def test_text_inline_underline(self):
        """Test text_inline with underline tag."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("Hello <u>underlined</u> world")
        assert result is doc

    def test_text_inline_strikethrough(self):
        """Test text_inline with strikethrough tag."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("Hello <strikethrough>struck</strikethrough> world")
        assert result is doc

    def test_text_inline_superscript(self):
        """Test text_inline with superscript tag."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("E = mc<sup>2</sup>")
        assert result is doc

    def test_text_inline_subscript(self):
        """Test text_inline with subscript tag."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("H<sub>2</sub>O")
        assert result is doc

    def test_text_inline_color(self):
        """Test text_inline with color tag."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline('Normal <color rgb="#FF0000">red</color> normal')
        assert result is doc

    def test_text_inline_font(self):
        """Test text_inline with font tag."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline('Default <font name="Courier">mono</font> default')
        assert result is doc

    def test_text_inline_link(self):
        """Test text_inline with link tag."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline('Click <a href="https://example.com">here</a>')
        assert result is doc

    def test_text_inline_br(self):
        """Test text_inline with br tag."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("Line 1<br>Line 2")
        assert result is doc

    def test_text_inline_entities(self):
        """Test text_inline with HTML entities."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("a &amp; b &lt; c &gt; d")
        assert result is doc

    def test_text_inline_nested_tags(self):
        """Test text_inline with nested tags."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline("<b>bold <i>bold-italic</i> bold</b>")
        assert result is doc

    def test_text_inline_complex(self):
        """Test text_inline with complex mixed markup."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_inline(
            '<b>Bold</b>, <i>italic</i>, <u>underline</u>, '
            '<color rgb="#FF0000">red</color>, '
            '<font name="Courier">mono</font>, '
            '<a href="https://example.com">link</a>'
        )
        assert result is doc

    def test_text_inline_empty(self):
        """Test text_inline with empty string."""
        doc = Document(margin=Margin.all(72))
        cursor_before = doc.cursor()
        doc.text_inline("")
        cursor_after = doc.cursor()
        assert abs(cursor_after - cursor_before) < 0.1

    def test_text_inline_advances_cursor(self):
        """Test that text_inline advances the cursor."""
        doc = Document(margin=Margin.all(72))
        cursor_before = doc.cursor()
        doc.text_inline("Hello <b>world</b>")
        cursor_after = doc.cursor()
        assert cursor_after < cursor_before


class TestTextWrapInline:
    """Test text_wrap_inline method (HTML-like inline formatting with wrapping)."""

    def test_text_wrap_inline_requires_margin(self):
        """Test that text_wrap_inline requires margin."""
        doc = Document()
        with pytest.raises(RuntimeError):
            doc.text_wrap_inline("Hello <b>world</b>")

    def test_text_wrap_inline_plain_text(self):
        """Test text_wrap_inline with plain text."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_wrap_inline("Hello world")
        assert result is doc

    def test_text_wrap_inline_with_tags(self):
        """Test text_wrap_inline with formatting tags."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_wrap_inline(
            "This is a <b>long</b> paragraph with <i>mixed</i> styles "
            "that should <u>wrap automatically</u> within the margins."
        )
        assert result is doc

    def test_text_wrap_inline_empty(self):
        """Test text_wrap_inline with empty string."""
        doc = Document(margin=Margin.all(72))
        cursor_before = doc.cursor()
        doc.text_wrap_inline("")
        cursor_after = doc.cursor()
        assert abs(cursor_after - cursor_before) < 0.1

    def test_text_wrap_inline_long_wraps_multiple_lines(self):
        """Test that long text actually wraps to multiple lines."""
        doc = Document(margin=Margin.all(72))
        cursor_before = doc.cursor()
        doc.text("Short")
        single_line_drop = cursor_before - doc.cursor()

        cursor_before_wrap = doc.cursor()
        doc.text_wrap_inline(
            "This is a <b>very long piece of text</b> that contains <i>many words</i> "
            "and should definitely <u>wrap across multiple lines</u> when rendered "
            "within the default page margins of the layout document."
        )
        multi_line_drop = cursor_before_wrap - doc.cursor()

        # Multi-line should drop more than single line
        assert multi_line_drop > single_line_drop * 1.5

    def test_text_wrap_inline_with_br(self):
        """Test text_wrap_inline with explicit line breaks."""
        doc = Document(margin=Margin.all(72))
        result = doc.text_wrap_inline("Line 1<br>Line 2<br/>Line 3")
        assert result is doc

    def test_text_wrap_inline_renders_pdf(self):
        """Test that text_wrap_inline produces valid PDF output."""
        doc = Document(margin=Margin.all(72))
        doc.text_inline("Hello <b>bold</b> and <i>italic</i>")
        doc.text_wrap_inline(
            "Wrapped <u>underline</u> text that goes on for a while."
        )
        pdf_bytes = doc.render()
        assert len(pdf_bytes) > 100
        assert pdf_bytes[:5] == b"%PDF-"
