/// Pinned upstream commit for the bundled Inter font file.
pub const INTER_UPSTREAM_COMMIT: &str = "353b61b9f4430d5f420d56605a6e7993e0941470";
/// Pinned upstream commit for the bundled Space Mono font file.
pub const SPACE_MONO_UPSTREAM_COMMIT: &str = "329858c2c4dbd3476f972a4ae00624b018cf4b81";
/// Bundled Inter variable TTF bytes.
pub const INTER_VARIABLE: &[u8] = include_bytes!("../assets/fonts/InterVariable.ttf");
/// Bundled Space Mono Regular TTF bytes.
pub const SPACE_MONO_REGULAR: &[u8] = include_bytes!("../assets/fonts/SpaceMono-Regular.ttf");

pub(crate) const INTER_FONTDB_FAMILY: &str = "Inter Variable";
