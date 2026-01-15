//! Image API types
//!
//! This module provides types for image embedding and positioning.

use std::borrow::Cow;

use crate::error::Result;

/// A source of image data that can be embedded in a PDF.
///
/// This trait allows the `image` method to accept multiple types:
/// - `&[u8]` - raw bytes (zero-copy)
/// - `Vec<u8>` - owned bytes
///
/// With the `std` feature (enabled by default), also supports:
/// - `&str` - file path
/// - `&Path` - file path
/// - `PathBuf` - owned file path
///
/// # Example
/// ```ignore
/// // From bytes (zero-copy)
/// doc.image(&bytes[..], [0.0, 0.0], 100.0, 100.0)?;
///
/// // From file path (requires "std" feature)
/// doc.image("photo.jpg", [0.0, 0.0], 100.0, 100.0)?;
/// ```
pub trait ImageSource<'a> {
    /// Loads the image data, returning borrowed or owned bytes.
    fn load(self) -> Result<Cow<'a, [u8]>>;
}

// Implementation for byte slices (zero-copy borrowed)
impl<'a> ImageSource<'a> for &'a [u8] {
    fn load(self) -> Result<Cow<'a, [u8]>> {
        Ok(Cow::Borrowed(self))
    }
}

// Implementation for owned bytes (works for any lifetime since it returns Owned)
impl<'a> ImageSource<'a> for Vec<u8> {
    fn load(self) -> Result<Cow<'a, [u8]>> {
        Ok(Cow::Owned(self))
    }
}

// File path implementations - only available with std feature (not in WASM)
#[cfg(feature = "std")]
mod path_impls {
    use super::*;
    use crate::error::Error;
    use std::path::Path;

    impl ImageSource<'static> for &str {
        fn load(self) -> Result<Cow<'static, [u8]>> {
            std::fs::read(self).map(Cow::Owned).map_err(Error::Io)
        }
    }

    impl ImageSource<'static> for &Path {
        fn load(self) -> Result<Cow<'static, [u8]>> {
            std::fs::read(self).map(Cow::Owned).map_err(Error::Io)
        }
    }

    impl ImageSource<'static> for std::path::PathBuf {
        fn load(self) -> Result<Cow<'static, [u8]>> {
            std::fs::read(&self).map(Cow::Owned).map_err(Error::Io)
        }
    }
}

/// Options for embedding and drawing images
#[derive(Debug, Clone, Default)]
pub struct ImageOptions {
    /// Position to draw the image (bottom-left corner)
    /// If None, uses (0, 0)
    pub at: Option<[f64; 2]>,

    /// Explicit width (overrides other sizing options)
    pub width: Option<f64>,

    /// Explicit height (overrides other sizing options)
    pub height: Option<f64>,

    /// Fit the image within these bounds while preserving aspect ratio
    /// Format: (max_width, max_height)
    pub fit: Option<(f64, f64)>,

    /// Scale factor (1.0 = original size)
    /// Applied after fit calculation if both are specified
    pub scale: Option<f64>,

    /// Position within the fit bounds
    pub position: Position,
}

impl ImageOptions {
    /// Creates options with explicit position and size
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        ImageOptions {
            at: Some([x, y]),
            width: Some(width),
            height: Some(height),
            ..Default::default()
        }
    }

    /// Creates options to fit image within bounds
    pub fn fit(max_width: f64, max_height: f64) -> Self {
        ImageOptions {
            fit: Some((max_width, max_height)),
            ..Default::default()
        }
    }

    /// Creates options to fit image within bounds at a specific position
    pub fn fit_at(x: f64, y: f64, max_width: f64, max_height: f64) -> Self {
        ImageOptions {
            at: Some([x, y]),
            fit: Some((max_width, max_height)),
            ..Default::default()
        }
    }

    /// Creates options with a scale factor
    pub fn scaled(scale: f64) -> Self {
        ImageOptions {
            scale: Some(scale),
            ..Default::default()
        }
    }

    /// Creates options at a position with original size
    pub fn at(x: f64, y: f64) -> Self {
        ImageOptions {
            at: Some([x, y]),
            ..Default::default()
        }
    }

    /// Sets the position
    pub fn with_position(mut self, pos: Position) -> Self {
        self.position = pos;
        self
    }

    /// Sets the scale factor
    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale = Some(scale);
        self
    }

    /// Sets the location
    pub fn with_at(mut self, x: f64, y: f64) -> Self {
        self.at = Some([x, y]);
        self
    }
}

/// Position/alignment for images within their bounds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// Center the image (default)
    #[default]
    Center,
    /// Align to top-left
    TopLeft,
    /// Align to top-center
    TopCenter,
    /// Align to top-right
    TopRight,
    /// Align to middle-left
    MiddleLeft,
    /// Align to middle-right
    MiddleRight,
    /// Align to bottom-left
    BottomLeft,
    /// Align to bottom-center
    BottomCenter,
    /// Align to bottom-right
    BottomRight,
}

impl Position {
    /// Calculates the offset for positioning an image within bounds
    ///
    /// Returns (x_offset, y_offset) to add to the base position
    pub fn calculate_offset(
        self,
        image_width: f64,
        image_height: f64,
        bounds_width: f64,
        bounds_height: f64,
    ) -> (f64, f64) {
        let x_offset = match self {
            Position::TopLeft | Position::MiddleLeft | Position::BottomLeft => 0.0,
            Position::TopCenter | Position::Center | Position::BottomCenter => {
                (bounds_width - image_width) / 2.0
            }
            Position::TopRight | Position::MiddleRight | Position::BottomRight => {
                bounds_width - image_width
            }
        };

        let y_offset = match self {
            Position::BottomLeft | Position::BottomCenter | Position::BottomRight => 0.0,
            Position::MiddleLeft | Position::Center | Position::MiddleRight => {
                (bounds_height - image_height) / 2.0
            }
            Position::TopLeft | Position::TopCenter | Position::TopRight => {
                bounds_height - image_height
            }
        };

        (x_offset, y_offset)
    }
}

/// Information about an embedded image
#[derive(Debug, Clone)]
pub struct EmbeddedImage {
    /// The image name (for use with draw_image)
    pub name: String,
    /// Original image width in pixels
    pub width: u32,
    /// Original image height in pixels
    pub height: u32,
}

impl EmbeddedImage {
    /// Returns the aspect ratio (width / height)
    pub fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }

    /// Calculates dimensions to fit within bounds while preserving aspect ratio
    pub fn fit_dimensions(&self, max_width: f64, max_height: f64) -> (f64, f64) {
        let aspect = self.aspect_ratio();
        let mut width = max_width;
        let mut height = width / aspect;

        if height > max_height {
            height = max_height;
            width = height * aspect;
        }

        (width, height)
    }

    /// Calculates dimensions with a scale factor
    pub fn scaled_dimensions(&self, scale: f64) -> (f64, f64) {
        (self.width as f64 * scale, self.height as f64 * scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_offset_center() {
        let (x, y) = Position::Center.calculate_offset(100.0, 50.0, 200.0, 100.0);
        assert_eq!(x, 50.0);
        assert_eq!(y, 25.0);
    }

    #[test]
    fn test_position_offset_top_left() {
        let (x, y) = Position::TopLeft.calculate_offset(100.0, 50.0, 200.0, 100.0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 50.0);
    }

    #[test]
    fn test_position_offset_bottom_right() {
        let (x, y) = Position::BottomRight.calculate_offset(100.0, 50.0, 200.0, 100.0);
        assert_eq!(x, 100.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn test_embedded_image_fit() {
        let img = EmbeddedImage {
            name: "test".to_string(),
            width: 800,
            height: 600,
        };

        // Fit into 400x400 box
        let (w, h) = img.fit_dimensions(400.0, 400.0);
        assert!((w - 400.0).abs() < 0.001);
        assert!((h - 300.0).abs() < 0.001);

        // Fit into 200x400 box (width constrained)
        let (w, h) = img.fit_dimensions(200.0, 400.0);
        assert!((w - 200.0).abs() < 0.001);
        assert!((h - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_embedded_image_scaled() {
        let img = EmbeddedImage {
            name: "test".to_string(),
            width: 100,
            height: 50,
        };

        let (w, h) = img.scaled_dimensions(2.0);
        assert_eq!(w, 200.0);
        assert_eq!(h, 100.0);

        let (w, h) = img.scaled_dimensions(0.5);
        assert_eq!(w, 50.0);
        assert_eq!(h, 25.0);
    }

    #[test]
    fn test_image_source_from_bytes_zero_copy() {
        use std::borrow::Cow;
        let bytes: &[u8] = &[1, 2, 3, 4];
        let result = bytes.load();
        assert!(result.is_ok());
        let cow = result.unwrap();
        // Verify it's borrowed (zero-copy), not owned
        assert!(matches!(cow, Cow::Borrowed(_)));
        assert_eq!(cow.as_ref(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_image_source_from_vec() {
        use std::borrow::Cow;
        let bytes: Vec<u8> = vec![1, 2, 3, 4];
        let result = bytes.load();
        assert!(result.is_ok());
        let cow = result.unwrap();
        // Vec should be owned
        assert!(matches!(cow, Cow::Owned(_)));
        assert_eq!(cow.as_ref(), &[1, 2, 3, 4]);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_image_source_from_invalid_path() {
        let path = "/nonexistent/path/to/image.png";
        let result = path.load();
        assert!(result.is_err());
    }
}
