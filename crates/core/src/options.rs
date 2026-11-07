//! User-tunable conversion settings.
//!
//! Options are grouped per converter family so the UI can show only the panel
//! that matters for the chosen target format.

/// All settings for a conversion. `Default` gives sensible values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConvertOptions {
    /// Settings used by the image converter.
    pub image: ImageOptions,
}

/// Settings for raster image output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageOptions {
    /// JPEG quality from 1 (smallest) to 100 (best). Values outside the range
    /// are clamped.
    pub jpeg_quality: u8,
    /// RGB colour painted behind transparent pixels when the target format has
    /// no alpha channel (JPEG, BMP).
    pub background: [u8; 3],
}

impl ImageOptions {
    /// Default JPEG quality: visually lossless for photos at a reasonable size.
    pub const DEFAULT_JPEG_QUALITY: u8 = 90;
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            jpeg_quality: Self::DEFAULT_JPEG_QUALITY,
            background: [255, 255, 255],
        }
    }
}
