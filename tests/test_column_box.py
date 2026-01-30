"""
Tests for pdfcrate column_box (multi-column layout) features.
"""

import pytest
from pdfcrate import Document, Margin


class TestColumnBox:
    """Test column_box method."""

    def test_column_box_requires_margin(self):
        """Test that column_box requires margin."""
        doc = Document()
        with pytest.raises(RuntimeError):
            doc.column_box(lambda d: None)

    def test_column_box_basic(self):
        """Test basic column_box with default 3 columns."""
        doc = Document(margin=Margin.all(72))
        doc.column_box(lambda d: d.text_wrap("Hello world from column box."))
        pdf = doc.render()
        assert len(pdf) > 100
        assert pdf[:5] == b"%PDF-"

    def test_column_box_two_columns(self):
        """Test column_box with 2 columns."""
        doc = Document(margin=Margin.all(72))
        doc.column_box(
            lambda d: d.text_wrap("Two column text content."),
            columns=2,
        )
        pdf = doc.render()
        assert pdf[:5] == b"%PDF-"

    def test_column_box_custom_spacer(self):
        """Test column_box with custom spacer."""
        doc = Document(margin=Margin.all(72))
        doc.column_box(
            lambda d: d.text_wrap("Custom spacer text."),
            columns=2,
            spacer=24.0,
        )
        pdf = doc.render()
        assert pdf[:5] == b"%PDF-"

    def test_column_box_returns_doc(self):
        """Test that column_box returns the document for chaining."""
        doc = Document(margin=Margin.all(72))
        result = doc.column_box(lambda d: d.text("Hello"))
        assert result is doc

    def test_column_box_with_text_wrap_inline(self):
        """Test column_box with rich text content."""
        doc = Document(margin=Margin.all(72))
        doc.column_box(
            lambda d: d.text_wrap_inline(
                "This is <b>bold</b> and <i>italic</i> text in columns."
            ),
            columns=2,
        )
        pdf = doc.render()
        assert pdf[:5] == b"%PDF-"

    def test_column_box_long_text_overflow(self):
        """Test that long text overflows to next column."""
        doc = Document(margin=Margin.all(72))
        long_text = " ".join(["Lorem ipsum dolor sit amet."] * 50)
        doc.column_box(
            lambda d: d.text_wrap(long_text),
            columns=3,
        )
        pdf = doc.render()
        assert pdf[:5] == b"%PDF-"

    def test_column_box_advances_cursor(self):
        """Test that column_box advances the cursor past the column area."""
        doc = Document(margin=Margin.all(72))
        cursor_before = doc.cursor()
        doc.column_box(
            lambda d: d.text_wrap("Some content in columns."),
            columns=2,
        )
        cursor_after = doc.cursor()
        assert cursor_after < cursor_before

    def test_column_box_multiple_calls(self):
        """Test multiple column_box calls on same document."""
        doc = Document(margin=Margin.all(72))
        doc.column_box(lambda d: d.text("First"), columns=2)
        doc.move_down(10)
        doc.column_box(lambda d: d.text("Second"), columns=3)
        pdf = doc.render()
        assert pdf[:5] == b"%PDF-"
