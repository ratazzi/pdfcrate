//! Interactive Forms (AcroForms) Demo
//!
//! Demonstrates pdfcrate's form field support:
//! - Text input fields
//! - Checkboxes (checked and unchecked)
//! - Dropdown select fields
//! - Form field counting
//!
//! Run with: cargo run --example forms_demo

use pdfcrate::prelude::{Document, LayoutDocument, Margin, PageLayout, PageSize};
use pdfcrate::Result as PdfResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Document::generate("forms_demo.pdf", |doc| {
        doc.title("Forms Demo").author("pdfcrate");
        add_page(doc)?;
        Ok(())
    })?;

    println!("Created: forms_demo.pdf");
    Ok(())
}

/// Adds the interactive forms demo page.
///
/// This function is also called by showcase.rs to include this page.
pub fn add_page(doc: &mut Document) -> PdfResult<()> {
    let (_page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    // Header band (top-left: 0, 842)
    doc.fill(|ctx| {
        ctx.gray(0.95).rectangle([0.0, 842.0], _page_width, 82.0);
    });

    doc.font("Helvetica").size(24.0);
    doc.text_at("Interactive Forms", [margin, 800.0]);

    doc.font("Helvetica").size(11.0);
    doc.text_at("Form field simulation", [margin, 780.0]);

    // Wrap into LayoutDocument for cursor-based body
    // Prawn: margin 36, indent(12) → text at 48
    let doc_owned = std::mem::take(doc);
    let mut layout = LayoutDocument::with_margin(
        doc_owned,
        Margin::new(142.0, margin - 12.0, margin - 12.0, margin - 12.0),
    );

    let abs_bottom = layout.bounds().absolute_bottom();

    layout.indent(12.0, 0.0, |layout| {
        // Prawn: label_x = 0, field_x = 120 (bounds-relative inside indent)
        let bounds_left = layout.bounds().absolute_left();
        let label_x = bounds_left;
        let field_x = bounds_left + 120.0;
        let field_width = 200.0;
        let field_height = 20.0;
        let row_height = 35.0;

        // Section: Contact Information
        layout.font("Helvetica").size(14.0);
        layout.text("Contact Information");
        layout.move_down(16.0);

        layout.font("Helvetica").size(11.0);

        // Name field
        let y = layout.cursor() + abs_bottom;
        layout.text_at("Name:", [label_x, y]);
        layout.add_text_field(
            "name",
            [field_x, y, field_x + field_width, y + field_height],
        );
        layout.move_down(row_height);

        // Email field
        let y = layout.cursor() + abs_bottom;
        layout.text_at("Email:", [label_x, y]);
        layout.add_text_field(
            "email",
            [field_x, y, field_x + field_width, y + field_height],
        );
        layout.move_down(row_height);

        // Phone field
        let y = layout.cursor() + abs_bottom;
        layout.text_at("Phone:", [label_x, y]);
        layout.add_text_field("phone", [field_x, y, field_x + 150.0, y + field_height]);
        layout.move_down(row_height + 20.0);

        // Section: Preferences
        layout.font("Helvetica").size(14.0);
        layout.text("Preferences");
        layout.move_down(16.0);

        layout.font("Helvetica").size(11.0);

        // Newsletter checkbox
        let y = layout.cursor() + abs_bottom;
        layout.text_at("Subscribe:", [label_x, y]);
        layout.add_checkbox("newsletter", [field_x, y, field_x + 18.0, y + 18.0], true);
        layout.text_at("Newsletter", [field_x + 22.0, y]);
        layout.move_down(row_height);

        // Updates checkbox
        let y = layout.cursor() + abs_bottom;
        layout.text_at("Receive:", [label_x, y]);
        layout.add_checkbox("updates", [field_x, y, field_x + 18.0, y + 18.0], false);
        layout.text_at("Product updates", [field_x + 22.0, y]);
        layout.move_down(row_height + 20.0);

        // Section: Selection
        layout.font("Helvetica").size(14.0);
        layout.text("Selection");
        layout.move_down(16.0);

        layout.font("Helvetica").size(11.0);

        // Country dropdown
        let y = layout.cursor() + abs_bottom;
        layout.text_at("Country:", [label_x, y]);
        layout.add_dropdown(
            "country",
            [field_x, y, field_x + field_width, y + field_height],
            vec!["USA", "Canada", "UK", "Germany", "France", "Japan"],
        );
        layout.move_down(row_height);

        // Department dropdown
        let y = layout.cursor() + abs_bottom;
        layout.text_at("Department:", [label_x, y]);
        layout.add_dropdown(
            "department",
            [field_x, y, field_x + 150.0, y + field_height],
            vec!["Sales", "Engineering", "Marketing", "Support"],
        );
    });

    *doc = layout.into_inner();
    Ok(())
}
