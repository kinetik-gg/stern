//! Deterministic render manifest.
//!
//! The manifest is a stable, hand-serialized JSON document: entry order
//! follows registry order, all numbers are formatted deterministically, and
//! pixel content is fingerprinted with FNV-1a 64 so two renders of the same
//! tree can be compared byte-for-byte.

/// One rendered variant recorded in the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    /// Story id.
    pub story_id: String,
    /// PNG file name inside the render directory.
    pub file: String,
    /// Logical width.
    pub logical_width: u32,
    /// Logical height.
    pub logical_height: u32,
    /// Scale factor times 100 (deterministic integer form).
    pub scale_percent: u32,
    /// Device pixel width.
    pub device_width: u32,
    /// Device pixel height.
    pub device_height: u32,
    /// FNV-1a 64 fingerprint of the straight-alpha RGBA pixel bytes.
    pub pixel_hash: u64,
}

/// FNV-1a 64-bit hash of a byte stream.
#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Serializes manifest entries into the deterministic JSON document.
#[must_use]
pub fn manifest_json(entries: &[ManifestEntry]) -> String {
    use std::fmt::Write as _;

    let mut out = String::from("{\n  \"generator\": \"stern-stories\",\n");
    out.push_str("  \"renderer\": \"cpu-tiny-skia\",\n  \"entries\": [\n");
    for (index, entry) in entries.iter().enumerate() {
        let _ = writeln!(
            out,
            "    {{\"story\": {:?}, \"file\": {:?}, \"logical\": [{}, {}], \
             \"scale_percent\": {}, \"device\": [{}, {}], \"pixel_hash\": \"{:016x}\"}}{}",
            entry.story_id,
            entry.file,
            entry.logical_width,
            entry.logical_height,
            entry.scale_percent,
            entry.device_width,
            entry.device_height,
            entry.pixel_hash,
            if index + 1 == entries.len() { "" } else { "," }
        );
    }
    out.push_str("  ]\n}\n");
    out
}

/// Extracts `(file, pixel_hash)` pairs from a manifest document produced by
/// [`manifest_json`].
///
/// This is a narrow line-oriented reader for the harness's own format, not a
/// general JSON parser.
#[must_use]
pub fn manifest_files(manifest: &str) -> Vec<(String, u64)> {
    let mut files = Vec::new();
    for line in manifest.lines() {
        let Some(file) = extract_string_field(line, "\"file\": \"") else {
            continue;
        };
        let Some(hash) = extract_string_field(line, "\"pixel_hash\": \"") else {
            continue;
        };
        if let Ok(hash) = u64::from_str_radix(&hash, 16) {
            files.push((file, hash));
        }
    }
    files
}

fn extract_string_field(line: &str, prefix: &str) -> Option<String> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}
