/// Pinned upstream commit for the bundled Inter font file.
pub const INTER_UPSTREAM_COMMIT: &str = "353b61b9f4430d5f420d56605a6e7993e0941470";
/// Pinned upstream commit for the bundled Geist Mono font file.
pub const GEIST_UPSTREAM_COMMIT: &str = "10dc7658f13c38a474cde201bb09a4617267545b";
/// Bundled Inter variable TTF bytes.
pub const INTER_VARIABLE: &[u8] = include_bytes!("../assets/fonts/InterVariable.ttf");
/// Bundled Geist Mono variable TTF bytes.
pub const GEIST_MONO_VARIABLE: &[u8] = include_bytes!("../assets/fonts/GeistMono-Variable.ttf");

pub(crate) const INTER_FONTDB_FAMILY: &str = "Inter Variable";
