//! Document outline (bookmarks) support for PDF documents
//!
//! This module provides types for creating a document outline (table of contents)
//! that appears in the PDF viewer's navigation panel.

use crate::api::link::DestinationFit;

/// An outline item (bookmark entry)
#[derive(Debug, Clone)]
pub struct OutlineItem {
    /// The title displayed in the outline
    pub title: String,
    /// The destination (page index and fit type)
    pub destination: Option<OutlineDestination>,
    /// Child items
    pub children: Vec<OutlineItem>,
    /// Whether this item starts closed (children hidden)
    pub closed: bool,
}

/// Destination for an outline item
#[derive(Debug, Clone)]
pub enum OutlineDestination {
    /// Page index with fit type
    Page {
        page_index: usize,
        fit: DestinationFit,
    },
    /// Named destination
    Named(String),
}

impl OutlineItem {
    /// Create a new outline item with a title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            destination: None,
            children: Vec::new(),
            closed: false,
        }
    }

    /// Create a new outline item with title and page destination
    pub fn page(title: impl Into<String>, page_index: usize) -> Self {
        Self {
            title: title.into(),
            destination: Some(OutlineDestination::Page {
                page_index,
                fit: DestinationFit::Fit,
            }),
            children: Vec::new(),
            closed: false,
        }
    }

    /// Create a new outline item with title and named destination
    pub fn named(title: impl Into<String>, dest_name: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            destination: Some(OutlineDestination::Named(dest_name.into())),
            children: Vec::new(),
            closed: false,
        }
    }

    /// Set the destination to a page index
    pub fn with_destination(mut self, page_index: usize) -> Self {
        self.destination = Some(OutlineDestination::Page {
            page_index,
            fit: DestinationFit::Fit,
        });
        self
    }

    /// Set the destination to a page with specific fit type
    pub fn with_destination_fit(mut self, page_index: usize, fit: DestinationFit) -> Self {
        self.destination = Some(OutlineDestination::Page { page_index, fit });
        self
    }

    /// Set the destination to a named destination
    pub fn with_named_destination(mut self, name: impl Into<String>) -> Self {
        self.destination = Some(OutlineDestination::Named(name.into()));
        self
    }

    /// Set whether this item starts closed
    pub fn with_closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
    }

    /// Add a child item
    pub fn add_child(&mut self, child: OutlineItem) -> &mut Self {
        self.children.push(child);
        self
    }

    /// Add a child item (builder pattern)
    pub fn with_child(mut self, child: OutlineItem) -> Self {
        self.children.push(child);
        self
    }
}

/// Builder for constructing document outlines using a closure-based DSL
///
/// # Example
///
/// ```ignore
/// doc.outline(|o| {
///     o.section("Chapter 1", 0, |o| {
///         o.page("Introduction", 0);
///         o.page("Getting Started", 1);
///     });
///     o.section("Chapter 2", 2, |o| {
///         o.page("Advanced Topics", 2);
///         o.section_closed("Subsection 2.1", 3, |o| {
///             o.page("Details", 3);
///         });
///     });
/// });
/// ```
#[derive(Default)]
pub struct OutlineBuilder {
    items: Vec<OutlineItem>,
}

impl OutlineBuilder {
    /// Create a new outline builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a section with children
    ///
    /// # Arguments
    ///
    /// * `title` - The section title
    /// * `page_index` - The destination page index
    /// * `f` - A closure to add child items
    pub fn section<F>(&mut self, title: impl Into<String>, page_index: usize, f: F) -> &mut Self
    where
        F: FnOnce(&mut OutlineBuilder),
    {
        let mut child_builder = OutlineBuilder::new();
        f(&mut child_builder);

        let mut item = OutlineItem::page(title, page_index);
        item.children = child_builder.items;
        self.items.push(item);
        self
    }

    /// Add a closed section with children (starts collapsed)
    pub fn section_closed<F>(
        &mut self,
        title: impl Into<String>,
        page_index: usize,
        f: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut OutlineBuilder),
    {
        let mut child_builder = OutlineBuilder::new();
        f(&mut child_builder);

        let mut item = OutlineItem::page(title, page_index);
        item.children = child_builder.items;
        item.closed = true;
        self.items.push(item);
        self
    }

    /// Add a section linking to a named destination
    pub fn section_named<F>(
        &mut self,
        title: impl Into<String>,
        dest_name: impl Into<String>,
        f: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut OutlineBuilder),
    {
        let mut child_builder = OutlineBuilder::new();
        f(&mut child_builder);

        let mut item = OutlineItem::named(title, dest_name);
        item.children = child_builder.items;
        self.items.push(item);
        self
    }

    /// Add a leaf page entry
    pub fn page(&mut self, title: impl Into<String>, page_index: usize) -> &mut Self {
        self.items.push(OutlineItem::page(title, page_index));
        self
    }

    /// Add a leaf page entry linking to a named destination
    pub fn page_named(
        &mut self,
        title: impl Into<String>,
        dest_name: impl Into<String>,
    ) -> &mut Self {
        self.items.push(OutlineItem::named(title, dest_name));
        self
    }

    /// Add a custom outline item
    pub fn item(&mut self, item: OutlineItem) -> &mut Self {
        self.items.push(item);
        self
    }

    /// Get the built items
    pub fn build(self) -> Vec<OutlineItem> {
        self.items
    }
}

/// Document outline (table of contents)
#[derive(Debug, Clone, Default)]
pub struct Outline {
    /// Root-level outline items
    pub items: Vec<OutlineItem>,
}

impl Outline {
    /// Create an empty outline
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Check if the outline is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Add a root-level item
    pub fn add(&mut self, item: OutlineItem) -> &mut Self {
        self.items.push(item);
        self
    }

    /// Add items from a builder
    pub fn add_from_builder(&mut self, builder: OutlineBuilder) -> &mut Self {
        self.items.extend(builder.items);
        self
    }

    /// Count total items (including all descendants)
    pub fn total_count(&self) -> usize {
        fn count_recursive(items: &[OutlineItem]) -> usize {
            items
                .iter()
                .map(|item| 1 + count_recursive(&item.children))
                .sum()
        }
        count_recursive(&self.items)
    }
}
