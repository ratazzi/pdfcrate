//! Form field appearance stream generation
//!
//! This module generates the visual appearance of form fields.

use super::{FieldType, FormField};
use crate::content::ContentBuilder;
use crate::objects::{PdfArray, PdfDict, PdfName, PdfObject, PdfRef, PdfStream};

/// Generates the normal appearance stream for a form field
pub fn generate_appearance(field: &FormField, font_ref: Option<PdfRef>) -> PdfStream {
    match field.field_type {
        FieldType::Text => generate_text_appearance(field, font_ref),
        FieldType::CheckBox => generate_checkbox_appearance(field, font_ref),
        FieldType::Choice => generate_choice_appearance(field, font_ref),
        _ => generate_empty_appearance(field),
    }
}

/// Generates appearance for a text field
fn generate_text_appearance(field: &FormField, font_ref: Option<PdfRef>) -> PdfStream {
    let width = field.rect[2] - field.rect[0];
    let height = field.rect[3] - field.rect[1];

    let mut content = ContentBuilder::new();

    // Background
    if let Some(ref bg) = field.background_color {
        content
            .set_fill_color_rgb(bg[0], bg[1], bg[2])
            .rect(0.0, 0.0, width, height)
            .fill();
    }

    // Border
    if let Some(ref bc) = field.border_color {
        content
            .set_stroke_color_rgb(bc[0], bc[1], bc[2])
            .set_line_width(1.0)
            .rect(0.5, 0.5, width - 1.0, height - 1.0)
            .stroke();
    }

    // Text content
    if let Some(ref value) = field.value {
        let font_size = if field.font_size == 0.0 {
            calculate_auto_font_size(height)
        } else {
            field.font_size
        };

        let text_y = (height - font_size) / 2.0 + 2.0; // Approximate vertical centering
        let text_x = 2.0; // Left padding

        content
            .begin_text()
            .set_fill_color_rgb(
                field.text_color[0],
                field.text_color[1],
                field.text_color[2],
            )
            .set_font(&field.font, font_size)
            .move_text_pos(text_x, text_y)
            .show_text(value)
            .end_text();
    }

    create_appearance_stream(content.build(), width, height, font_ref, &field.font)
}

/// Generates appearance for a checkbox field
fn generate_checkbox_appearance(field: &FormField, font_ref: Option<PdfRef>) -> PdfStream {
    let width = field.rect[2] - field.rect[0];
    let height = field.rect[3] - field.rect[1];
    let is_checked = field.value.as_deref() == Some("Yes");

    let mut content = ContentBuilder::new();

    // Background
    if let Some(ref bg) = field.background_color {
        content
            .set_fill_color_rgb(bg[0], bg[1], bg[2])
            .rect(0.0, 0.0, width, height)
            .fill();
    }

    // Border
    if let Some(ref bc) = field.border_color {
        content
            .set_stroke_color_rgb(bc[0], bc[1], bc[2])
            .set_line_width(1.0)
            .rect(0.5, 0.5, width - 1.0, height - 1.0)
            .stroke();
    }

    // Checkmark if checked
    if is_checked {
        let check_size = width.min(height) * 0.6;
        let x_offset = (width - check_size) / 2.0;
        let y_offset = (height - check_size) / 2.0;

        content
            .set_stroke_color_rgb(0.0, 0.0, 0.0)
            .set_line_width(2.0)
            // Draw checkmark
            .move_to(x_offset, y_offset + check_size * 0.5)
            .line_to(x_offset + check_size * 0.35, y_offset)
            .line_to(x_offset + check_size, y_offset + check_size)
            .stroke();
    }

    create_appearance_stream(content.build(), width, height, font_ref, &field.font)
}

/// Generates appearance for a choice field (dropdown/listbox)
fn generate_choice_appearance(field: &FormField, font_ref: Option<PdfRef>) -> PdfStream {
    let width = field.rect[2] - field.rect[0];
    let height = field.rect[3] - field.rect[1];

    let mut content = ContentBuilder::new();

    // Background
    if let Some(ref bg) = field.background_color {
        content
            .set_fill_color_rgb(bg[0], bg[1], bg[2])
            .rect(0.0, 0.0, width, height)
            .fill();
    }

    // Border
    if let Some(ref bc) = field.border_color {
        content
            .set_stroke_color_rgb(bc[0], bc[1], bc[2])
            .set_line_width(1.0)
            .rect(0.5, 0.5, width - 1.0, height - 1.0)
            .stroke();
    }

    // Selected value text
    if let Some(ref value) = field.value {
        let font_size = if field.font_size == 0.0 {
            calculate_auto_font_size(height)
        } else {
            field.font_size
        };

        let text_y = (height - font_size) / 2.0 + 2.0;
        let text_x = 2.0;

        content
            .begin_text()
            .set_fill_color_rgb(
                field.text_color[0],
                field.text_color[1],
                field.text_color[2],
            )
            .set_font(&field.font, font_size)
            .move_text_pos(text_x, text_y)
            .show_text(value)
            .end_text();
    }

    // Dropdown arrow for combo boxes
    if field.flags.has(super::FieldFlags::COMBO) {
        let arrow_size = height * 0.3;
        let arrow_x = width - height + (height - arrow_size) / 2.0;
        let arrow_y = (height - arrow_size) / 2.0;

        // Draw dropdown button area
        content
            .set_fill_color_rgb(0.9, 0.9, 0.9)
            .rect(width - height, 0.0, height, height)
            .fill();

        // Draw arrow
        content
            .set_fill_color_rgb(0.3, 0.3, 0.3)
            .move_to(arrow_x, arrow_y + arrow_size)
            .line_to(arrow_x + arrow_size, arrow_y + arrow_size)
            .line_to(arrow_x + arrow_size / 2.0, arrow_y)
            .close_path()
            .fill();
    }

    create_appearance_stream(content.build(), width, height, font_ref, &field.font)
}

/// Generates an empty appearance (placeholder)
fn generate_empty_appearance(field: &FormField) -> PdfStream {
    let width = field.rect[2] - field.rect[0];
    let height = field.rect[3] - field.rect[1];

    let mut content = ContentBuilder::new();

    // Just draw a border
    if let Some(ref bc) = field.border_color {
        content
            .set_stroke_color_rgb(bc[0], bc[1], bc[2])
            .set_line_width(1.0)
            .rect(0.5, 0.5, width - 1.0, height - 1.0)
            .stroke();
    }

    create_appearance_stream(content.build(), width, height, None, &field.font)
}

/// Creates an appearance stream with proper dictionary
fn create_appearance_stream(
    content_data: Vec<u8>,
    width: f64,
    height: f64,
    font_ref: Option<PdfRef>,
    font_name: &str,
) -> PdfStream {
    let mut stream = PdfStream::from_data_compressed(content_data);
    let dict = stream.dict_mut();

    // Type
    dict.set("Type", PdfObject::Name(PdfName::new("XObject")));
    dict.set("Subtype", PdfObject::Name(PdfName::new("Form")));

    // BBox
    let bbox = PdfArray::from(vec![
        PdfObject::Integer(0),
        PdfObject::Integer(0),
        PdfObject::Real(width),
        PdfObject::Real(height),
    ]);
    dict.set("BBox", PdfObject::Array(bbox));

    // Resources
    let mut resources = PdfDict::new();

    if let Some(fref) = font_ref {
        let mut font_dict = PdfDict::new();
        font_dict.set(font_name, PdfObject::Reference(fref));
        resources.set("Font", PdfObject::Dict(font_dict));
    }

    dict.set("Resources", PdfObject::Dict(resources));

    stream
}

/// Calculates automatic font size based on field height
fn calculate_auto_font_size(height: f64) -> f64 {
    // Leave some padding
    (height - 4.0).clamp(6.0, 24.0)
}

/// Generates appearance dictionary with multiple states (for checkboxes)
pub fn generate_checkbox_appearances(
    field: &FormField,
    font_ref: Option<PdfRef>,
) -> (PdfStream, PdfStream) {
    // Normal state (unchecked)
    let mut field_off = field.clone();
    field_off.value = Some("Off".to_string());
    let ap_off = generate_checkbox_appearance(&field_off, font_ref);

    // Yes state (checked)
    let mut field_yes = field.clone();
    field_yes.value = Some("Yes".to_string());
    let ap_yes = generate_checkbox_appearance(&field_yes, font_ref);

    (ap_off, ap_yes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_text_appearance() {
        let field = FormField::text("test", [0.0, 0.0, 200.0, 20.0]).with_value("Hello");

        let stream = generate_text_appearance(&field, None);
        assert!(!stream.data().is_empty());
    }

    #[test]
    fn test_generate_checkbox_appearance() {
        let field = FormField::checkbox("check", [0.0, 0.0, 20.0, 20.0], true);

        let stream = generate_checkbox_appearance(&field, None);
        assert!(!stream.data().is_empty());
    }

    #[test]
    fn test_auto_font_size() {
        assert_eq!(calculate_auto_font_size(20.0), 16.0);
        assert_eq!(calculate_auto_font_size(10.0), 6.0);
        assert_eq!(calculate_auto_font_size(50.0), 24.0);
    }
}
