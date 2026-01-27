//! Interactive Forms (AcroForms) Demo
//!
//! Demonstrates pdfcrate's form field support:
//! - Text input fields
//! - Checkboxes (checked and unchecked)
//! - Dropdown select fields
//! - Form field counting
//!
//! Run with: cargo run --example forms_demo

use pdfcrate::prelude::{Document, PageLayout, PageSize};
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
    let (page_width, _page_height) = PageSize::A4.dimensions(PageLayout::Portrait);
    let margin = 48.0;

    // Header band (top-left: 0, 842)
    doc.fill(|ctx| {
        ctx.gray(0.95).rect_tl([0.0, 842.0], page_width, 82.0);
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
