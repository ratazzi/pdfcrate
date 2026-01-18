//! Link annotation support for PDF documents
//!
//! This module provides types for creating clickable links in PDF documents.
//! Links can point to external URLs, internal destinations, or local files.

use crate::objects::{PdfArray, PdfDict, PdfName, PdfObject, PdfRef, PdfString};

/// The action type for a link annotation
#[derive(Debug, Clone)]
pub enum LinkAction {
    /// External URL link (opens in browser)
    Uri(String),
    /// Internal destination (page jump within document)
    Destination(LinkDestination),
    /// Launch external file or application
    Launch { path: String, new_window: bool },
}

/// Internal link destination types
#[derive(Debug, Clone)]
pub enum LinkDestination {
    /// Named destination (string identifier)
    Named(String),
    /// Page reference with fit type
    Page {
        page_index: usize,
        fit: DestinationFit,
    },
}

/// How to fit the destination page in the viewer
#[derive(Debug, Clone, Default)]
pub enum DestinationFit {
    /// Fit the page in the window (default)
    #[default]
    Fit,
    /// Fit the width of the page
    FitH(Option<f64>),
    /// Fit the height of the page
    FitV(Option<f64>),
    /// Fit a specific rectangle
    FitR {
        left: f64,
        bottom: f64,
        right: f64,
        top: f64,
    },
    /// Display at specific position and zoom
    XYZ {
        left: Option<f64>,
        top: Option<f64>,
        zoom: Option<f64>,
    },
}

/// A link annotation that creates a clickable region in the PDF
#[derive(Debug, Clone)]
pub struct LinkAnnotation {
    /// The clickable rectangle [x1, y1, x2, y2] in page coordinates
    pub rect: [f64; 4],
    /// The link action to perform when clicked
    pub action: LinkAction,
    /// Border style [horizontal_radius, vertical_radius, width]
    /// Default is [0, 0, 0] (no visible border)
    pub border: [f64; 3],
    /// Optional highlight mode when clicked
    pub highlight: Option<HighlightMode>,
}

/// Highlight mode when link is clicked
#[derive(Debug, Clone, Copy, Default)]
pub enum HighlightMode {
    /// No highlighting
    None,
    /// Invert the contents of the annotation rectangle
    #[default]
    Invert,
    /// Invert the border of the annotation
    Outline,
    /// Display the annotation as if it were being pushed
    Push,
}

impl LinkAnnotation {
    /// Create a new URL link annotation
    pub fn url(rect: [f64; 4], url: impl Into<String>) -> Self {
        Self {
            rect,
            action: LinkAction::Uri(url.into()),
            border: [0.0, 0.0, 0.0],
            highlight: None,
        }
    }

    /// Create a new internal destination link
    pub fn destination(rect: [f64; 4], dest: LinkDestination) -> Self {
        Self {
            rect,
            action: LinkAction::Destination(dest),
            border: [0.0, 0.0, 0.0],
            highlight: None,
        }
    }

    /// Create a link to a named destination
    pub fn named(rect: [f64; 4], name: impl Into<String>) -> Self {
        Self::destination(rect, LinkDestination::Named(name.into()))
    }

    /// Create a link to a specific page
    pub fn page(rect: [f64; 4], page_index: usize) -> Self {
        Self::destination(
            rect,
            LinkDestination::Page {
                page_index,
                fit: DestinationFit::Fit,
            },
        )
    }

    /// Create a link to launch a local file
    pub fn launch(rect: [f64; 4], path: impl Into<String>, new_window: bool) -> Self {
        Self {
            rect,
            action: LinkAction::Launch {
                path: path.into(),
                new_window,
            },
            border: [0.0, 0.0, 0.0],
            highlight: None,
        }
    }

    /// Set the border style
    pub fn with_border(mut self, horizontal: f64, vertical: f64, width: f64) -> Self {
        self.border = [horizontal, vertical, width];
        self
    }

    /// Set the highlight mode
    pub fn with_highlight(mut self, mode: HighlightMode) -> Self {
        self.highlight = Some(mode);
        self
    }

    /// Convert to a PDF dictionary object
    ///
    /// # Arguments
    /// * `page_ref` - Optional reference to the page containing this annotation
    /// * `page_refs` - Optional list of all page references for resolving internal page links
    pub fn to_dict(&self, page_ref: Option<PdfRef>, page_refs: Option<&[PdfRef]>) -> PdfDict {
        let mut dict = PdfDict::new();

        // Required fields
        dict.set("Type", PdfObject::Name(PdfName::new("Annot")));
        dict.set("Subtype", PdfObject::Name(PdfName::new("Link")));

        // Rectangle
        let rect = PdfArray::from(vec![
            PdfObject::Real(self.rect[0]),
            PdfObject::Real(self.rect[1]),
            PdfObject::Real(self.rect[2]),
            PdfObject::Real(self.rect[3]),
        ]);
        dict.set("Rect", PdfObject::Array(rect));

        // Border
        let border = PdfArray::from(vec![
            PdfObject::Real(self.border[0]),
            PdfObject::Real(self.border[1]),
            PdfObject::Real(self.border[2]),
        ]);
        dict.set("Border", PdfObject::Array(border));

        // Page reference (optional)
        if let Some(pref) = page_ref {
            dict.set("P", PdfObject::Reference(pref));
        }

        // Highlight mode
        if let Some(highlight) = &self.highlight {
            let h = match highlight {
                HighlightMode::None => "N",
                HighlightMode::Invert => "I",
                HighlightMode::Outline => "O",
                HighlightMode::Push => "P",
            };
            dict.set("H", PdfObject::Name(PdfName::new(h)));
        }

        // Action or Destination
        match &self.action {
            LinkAction::Uri(url) => {
                let mut action = PdfDict::new();
                action.set("Type", PdfObject::Name(PdfName::new("Action")));
                action.set("S", PdfObject::Name(PdfName::new("URI")));
                action.set("URI", PdfObject::String(PdfString::from(url.as_str())));
                dict.set("A", PdfObject::Dict(action));
            }
            LinkAction::Destination(dest) => match dest {
                LinkDestination::Named(name) => {
                    dict.set("Dest", PdfObject::String(PdfString::from_text(name)));
                }
                LinkDestination::Page { page_index, fit } => {
                    // Resolve page index to page reference if available
                    let mut dest_array = PdfArray::new();
                    let page_obj = if let Some(refs) = page_refs {
                        refs.get(*page_index)
                            .map(|r| PdfObject::Reference(*r))
                            .unwrap_or(PdfObject::Integer(*page_index as i64))
                    } else {
                        // Fallback to integer (won't work in PDF viewers)
                        PdfObject::Integer(*page_index as i64)
                    };
                    dest_array.push(page_obj);
                    match fit {
                        DestinationFit::Fit => {
                            dest_array.push(PdfObject::Name(PdfName::new("Fit")));
                        }
                        DestinationFit::FitH(top) => {
                            dest_array.push(PdfObject::Name(PdfName::new("FitH")));
                            dest_array.push(match top {
                                Some(t) => PdfObject::Real(*t),
                                None => PdfObject::Null,
                            });
                        }
                        DestinationFit::FitV(left) => {
                            dest_array.push(PdfObject::Name(PdfName::new("FitV")));
                            dest_array.push(match left {
                                Some(l) => PdfObject::Real(*l),
                                None => PdfObject::Null,
                            });
                        }
                        DestinationFit::FitR {
                            left,
                            bottom,
                            right,
                            top,
                        } => {
                            dest_array.push(PdfObject::Name(PdfName::new("FitR")));
                            dest_array.push(PdfObject::Real(*left));
                            dest_array.push(PdfObject::Real(*bottom));
                            dest_array.push(PdfObject::Real(*right));
                            dest_array.push(PdfObject::Real(*top));
                        }
                        DestinationFit::XYZ { left, top, zoom } => {
                            dest_array.push(PdfObject::Name(PdfName::new("XYZ")));
                            dest_array.push(match left {
                                Some(l) => PdfObject::Real(*l),
                                None => PdfObject::Null,
                            });
                            dest_array.push(match top {
                                Some(t) => PdfObject::Real(*t),
                                None => PdfObject::Null,
                            });
                            dest_array.push(match zoom {
                                Some(z) => PdfObject::Real(*z),
                                None => PdfObject::Null,
                            });
                        }
                    }
                    dict.set("Dest", PdfObject::Array(dest_array));
                }
            },
            LinkAction::Launch { path, new_window } => {
                let mut action = PdfDict::new();
                action.set("Type", PdfObject::Name(PdfName::new("Action")));
                action.set("S", PdfObject::Name(PdfName::new("Launch")));
                action.set("F", PdfObject::String(PdfString::from_text(path)));
                if *new_window {
                    action.set("NewWindow", PdfObject::Bool(true));
                }
                dict.set("A", PdfObject::Dict(action));
            }
        }

        dict
    }
}
