//! Page sizes and layouts

use super::measurements::{cm, inch, mm};

/// Standard page sizes
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PageSize {
    /// A4: 210mm x 297mm (595.28 x 841.89 points)
    #[default]
    A4,
    /// A3: 297mm x 420mm (841.89 x 1190.55 points)
    A3,
    /// A5: 148mm x 210mm (419.53 x 595.28 points)
    A5,
    /// US Letter: 8.5in x 11in (612 x 792 points)
    Letter,
    /// US Legal: 8.5in x 14in (612 x 1008 points)
    Legal,
    /// US Tabloid: 11in x 17in (792 x 1224 points)
    Tabloid,
    /// Custom size in points
    Custom(f64, f64),
}

impl PageSize {
    /// Returns the page dimensions in points as (width, height)
    pub fn dimensions(&self, layout: PageLayout) -> (f64, f64) {
        let (w, h) = match self {
            PageSize::A4 => (595.28, 841.89),
            PageSize::A3 => (841.89, 1190.55),
            PageSize::A5 => (419.53, 595.28),
            PageSize::Letter => (612.0, 792.0),
            PageSize::Legal => (612.0, 1008.0),
            PageSize::Tabloid => (792.0, 1224.0),
            PageSize::Custom(w, h) => (*w, *h),
        };

        match layout {
            PageLayout::Portrait => (w, h),
            PageLayout::Landscape => (h, w),
        }
    }

    /// Creates a custom page size from millimeters
    pub fn from_mm(width: f64, height: f64) -> Self {
        PageSize::Custom(mm(width), mm(height))
    }

    /// Creates a custom page size from centimeters
    pub fn from_cm(width: f64, height: f64) -> Self {
        PageSize::Custom(cm(width), cm(height))
    }

    /// Creates a custom page size from inches
    pub fn from_inches(width: f64, height: f64) -> Self {
        PageSize::Custom(inch(width), inch(height))
    }
}

/// Page layout orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageLayout {
    /// Portrait orientation (taller than wide)
    #[default]
    Portrait,
    /// Landscape orientation (wider than tall)
    Landscape,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_size_dimensions() {
        let (w, h) = PageSize::A4.dimensions(PageLayout::Portrait);
        assert!((w - 595.28).abs() < 0.01);
        assert!((h - 841.89).abs() < 0.01);
    }

    #[test]
    fn test_page_size_landscape() {
        let (w, h) = PageSize::A4.dimensions(PageLayout::Landscape);
        assert!((w - 841.89).abs() < 0.01);
        assert!((h - 595.28).abs() < 0.01);
    }

    #[test]
    fn test_page_size_from_mm() {
        let size = PageSize::from_mm(210.0, 297.0);
        let (w, h) = size.dimensions(PageLayout::Portrait);
        // A4 is 210mm x 297mm
        assert!((w - 595.28).abs() < 1.0);
        assert!((h - 841.89).abs() < 1.0);
    }

    #[test]
    fn test_page_size_from_inches() {
        let size = PageSize::from_inches(8.5, 11.0);
        let (w, h) = size.dimensions(PageLayout::Portrait);
        // US Letter is 8.5in x 11in
        assert_eq!(w, 612.0);
        assert_eq!(h, 792.0);
    }
}
