//! Pinned archive loading and provenance verification.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    io::{Cursor, Read},
    path::Path,
};

use flate2::read::GzDecoder;
use serde_json::Value;
use sha2::{Digest, Sha256, Sha512};

use crate::{Error, ErrorKind, Result};

/// Exact SHA-256 of the published `@phosphor-icons/core` 2.1.1 archive.
pub const EXPECTED_SHA256: &str =
    "313332be6190b724da24107addd781799b48bf76b13963f24501112ffe1baadd";
/// Exact npm SHA-512 integrity of the published archive.
pub const EXPECTED_NPM_INTEGRITY: &str = "sha512-v4ARvrip4qBCImOE5rmPUylOEK4iiED9ZyKjcvzuezqMaiRASCHKcRIuvvxL/twvLpkfnEODCOJp5dM4eZilxQ==";
const EXPECTED_PACKAGE: &str = "@phosphor-icons/core";
const EXPECTED_VERSION: &str = "2.1.1";
const EXPECTED_SOURCE_URL: &str =
    "https://registry.npmjs.org/@phosphor-icons/core/-/core-2.1.1.tgz";
const EXPECTED_ENTRIES: usize = 9_081;
const PACKAGE_JSON: &str = "package/package.json";
const LICENSE_PATH: &str = "package/LICENSE";
const CATALOG_PATH: &str = "package/dist/icons.d.ts";
const TYPES_PATH: &str = "package/dist/types.d.ts";
const EXPECTED_LICENSE: &str = include_str!("../../../third-party/phosphor/LICENSE");

/// Verified immutable source metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotProvenance {
    /// Published package name.
    pub package: String,
    /// Exact published version.
    pub version: String,
    /// Official archive URL.
    pub source_url: String,
    /// Lowercase SHA-256 digest.
    pub sha256: String,
    /// npm-compatible SHA-512 Subresource Integrity value.
    pub npm_integrity: String,
    /// Tar entry count, including upstream duplicate spellings.
    pub archive_entries: usize,
    /// SPDX license identifier.
    pub license: String,
    /// Whether upstream's declared `IconEntry` schema contains RTL metadata.
    pub has_rtl_metadata: bool,
}

/// An explicitly loaded and verified offline package snapshot.
#[derive(Clone, Debug)]
pub struct Snapshot {
    entries: BTreeMap<String, Vec<u8>>,
    provenance: SnapshotProvenance,
}

impl Snapshot {
    /// Reads and verifies a snapshot from disk.
    ///
    /// # Errors
    ///
    /// Returns a structured I/O, archive, or provenance error when the source
    /// is unavailable, malformed, or differs from the exact pinned release.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| Error::io(path, &error))?;
        Self::from_bytes(&bytes)
    }

    /// Verifies source archive bytes. Useful for deterministic tooling and tests.
    ///
    /// # Errors
    ///
    /// Returns a structured archive or provenance error on any mismatch.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let sha256 = sha256_hex(bytes);
        let npm_integrity = npm_integrity(bytes);
        verify_integrity(&sha256, &npm_integrity)?;
        Self::from_verified_bytes(bytes, sha256, npm_integrity)
    }

    fn from_verified_bytes(bytes: &[u8], sha256: String, npm_integrity: String) -> Result<Self> {
        let decoder = GzDecoder::new(Cursor::new(bytes));
        let mut archive = tar::Archive::new(decoder);
        let mut entries = BTreeMap::new();
        let mut archive_entries = 0_usize;
        let iter = archive
            .entries()
            .map_err(|error| Error::new(ErrorKind::Archive, "archive", error.to_string()))?;
        for entry in iter {
            let mut entry = entry.map_err(|error| {
                Error::new(ErrorKind::Archive, "archive.entry", error.to_string())
            })?;
            let path = entry
                .path()
                .map_err(|error| {
                    Error::new(ErrorKind::Archive, "archive.entry.path", error.to_string())
                })?
                .to_string_lossy()
                .replace('\\', "/");
            let mut data = Vec::new();
            entry
                .read_to_end(&mut data)
                .map_err(|error| Error::new(ErrorKind::Archive, &path, error.to_string()))?;
            entries.insert(path, data);
            archive_entries += 1;
        }
        Self::verify_entries(entries, archive_entries, sha256, npm_integrity)
    }

    fn verify_entries(
        entries: BTreeMap<String, Vec<u8>>,
        archive_entries: usize,
        sha256: String,
        npm_integrity: String,
    ) -> Result<Self> {
        if archive_entries != EXPECTED_ENTRIES {
            return Err(Error::new(
                ErrorKind::Provenance,
                "archive.entries",
                format!("expected {EXPECTED_ENTRIES}, found {archive_entries}"),
            ));
        }
        let package_text = utf8_entry(&entries, PACKAGE_JSON)?;
        let package: Value = serde_json::from_str(package_text)
            .map_err(|error| Error::new(ErrorKind::Provenance, PACKAGE_JSON, error.to_string()))?;
        require_json(&package, "name", EXPECTED_PACKAGE)?;
        require_json(&package, "version", EXPECTED_VERSION)?;
        require_json(&package, "license", "MIT")?;

        let license = utf8_entry(&entries, LICENSE_PATH)?;
        if normalize_newlines(license).trim_end() != normalize_newlines(EXPECTED_LICENSE).trim_end()
        {
            return Err(Error::new(
                ErrorKind::Provenance,
                LICENSE_PATH,
                "MIT license text or attribution differs from the pinned copy",
            ));
        }

        let catalog = utf8_entry(&entries, CATALOG_PATH)?;
        if !catalog.contains("export declare const icons: readonly [{") {
            return Err(Error::new(
                ErrorKind::Provenance,
                CATALOG_PATH,
                "canonical icons tuple declaration is missing",
            ));
        }
        let types = utf8_entry(&entries, TYPES_PATH)?;
        let schema_start = types.find("export interface IconEntry {").ok_or_else(|| {
            Error::new(
                ErrorKind::Provenance,
                TYPES_PATH,
                "IconEntry schema is missing",
            )
        })?;
        let schema_tail = &types[schema_start..];
        let schema_end = schema_tail.find("\n}").ok_or_else(|| {
            Error::new(
                ErrorKind::Provenance,
                TYPES_PATH,
                "IconEntry schema is unterminated",
            )
        })?;
        let schema = &schema_tail[..schema_end];
        let has_rtl_metadata = schema
            .lines()
            .any(|line| line.to_ascii_lowercase().contains("rtl"));
        if has_rtl_metadata {
            return Err(Error::new(
                ErrorKind::Provenance,
                "IconEntry.rtl",
                "pinned schema unexpectedly declares RTL metadata",
            ));
        }

        Ok(Self {
            entries,
            provenance: SnapshotProvenance {
                package: EXPECTED_PACKAGE.to_owned(),
                version: EXPECTED_VERSION.to_owned(),
                source_url: EXPECTED_SOURCE_URL.to_owned(),
                sha256,
                npm_integrity,
                archive_entries,
                license: "MIT".to_owned(),
                has_rtl_metadata,
            },
        })
    }

    /// Returns verified source metadata.
    #[must_use]
    pub fn provenance(&self) -> &SnapshotProvenance {
        &self.provenance
    }

    /// Returns an archive entry as bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Archive`] when the entry is absent.
    pub fn entry(&self, path: &str) -> Result<&[u8]> {
        self.entries
            .get(path)
            .map(Vec::as_slice)
            .ok_or_else(|| Error::new(ErrorKind::Archive, path, "required entry is missing"))
    }

    /// Returns a UTF-8 archive entry.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Archive`] when the entry is absent or not UTF-8.
    pub fn text(&self, path: &str) -> Result<&str> {
        std::str::from_utf8(self.entry(path)?)
            .map_err(|error| Error::new(ErrorKind::Archive, path, error.to_string()))
    }

    pub(crate) fn paths(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

fn require_json(value: &Value, field: &str, expected: &str) -> Result<()> {
    let actual = value.get(field).and_then(Value::as_str).ok_or_else(|| {
        Error::new(
            ErrorKind::Provenance,
            format!("{PACKAGE_JSON}.{field}"),
            "expected a string",
        )
    })?;
    if actual != expected {
        return Err(Error::new(
            ErrorKind::Provenance,
            format!("{PACKAGE_JSON}.{field}"),
            format!("expected `{expected}`, found `{actual}`"),
        ));
    }
    Ok(())
}

fn utf8_entry<'a>(entries: &'a BTreeMap<String, Vec<u8>>, path: &str) -> Result<&'a str> {
    let bytes = entries
        .get(path)
        .ok_or_else(|| Error::new(ErrorKind::Archive, path, "required entry is missing"))?;
    std::str::from_utf8(bytes)
        .map_err(|error| Error::new(ErrorKind::Archive, path, error.to_string()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        })
}

fn npm_integrity(bytes: &[u8]) -> String {
    format!("sha512-{}", base64(&Sha512::digest(bytes)))
}

fn verify_integrity(sha256: &str, npm_integrity: &str) -> Result<()> {
    if sha256 != EXPECTED_SHA256 {
        return Err(Error::new(
            ErrorKind::Provenance,
            "archive.sha256",
            format!("expected {EXPECTED_SHA256}, found {sha256}"),
        ));
    }
    if npm_integrity != EXPECTED_NPM_INTEGRITY {
        return Err(Error::new(
            ErrorKind::Provenance,
            "archive.npm_integrity",
            format!("expected {EXPECTED_NPM_INTEGRITY}, found {npm_integrity}"),
        ));
    }
    Ok(())
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        output.push(char::from(ALPHABET[((value >> 18) & 63) as usize]));
        output.push(char::from(ALPHABET[((value >> 12) & 63) as usize]));
        output.push(if chunk.len() > 1 {
            char::from(ALPHABET[((value >> 6) & 63) as usize])
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            char::from(ALPHABET[(value & 63) as usize])
        } else {
            '='
        });
    }
    output
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimum_entries(package: &str, license: &str) -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([
            (PACKAGE_JSON.to_owned(), package.as_bytes().to_vec()),
            (LICENSE_PATH.to_owned(), license.as_bytes().to_vec()),
            (
                CATALOG_PATH.to_owned(),
                b"export declare const icons: readonly [{\n}];\n".to_vec(),
            ),
            (
                TYPES_PATH.to_owned(),
                b"export interface IconEntry {\n    name: string;\n}\n".to_vec(),
            ),
        ])
    }

    #[test]
    fn rejects_wrong_version_and_license_after_archive_integrity() {
        let wrong_version = minimum_entries(
            r#"{"name":"@phosphor-icons/core","version":"2.2.0","license":"MIT"}"#,
            EXPECTED_LICENSE,
        );
        let error = Snapshot::verify_entries(
            wrong_version,
            EXPECTED_ENTRIES,
            EXPECTED_SHA256.to_owned(),
            EXPECTED_NPM_INTEGRITY.to_owned(),
        )
        .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Provenance);
        assert!(error.context.ends_with(".version"));

        let wrong_license = minimum_entries(
            r#"{"name":"@phosphor-icons/core","version":"2.1.1","license":"MIT"}"#,
            "MIT License\nwrong attribution\n",
        );
        let error = Snapshot::verify_entries(
            wrong_license,
            EXPECTED_ENTRIES,
            EXPECTED_SHA256.to_owned(),
            EXPECTED_NPM_INTEGRITY.to_owned(),
        )
        .unwrap_err();
        assert_eq!(error.context, LICENSE_PATH);
    }

    #[test]
    fn rejects_unexpected_rtl_schema_field() {
        let mut entries = minimum_entries(
            r#"{"name":"@phosphor-icons/core","version":"2.1.1","license":"MIT"}"#,
            EXPECTED_LICENSE,
        );
        entries.insert(
            TYPES_PATH.to_owned(),
            b"export interface IconEntry {\n    rtl: boolean;\n}\n".to_vec(),
        );
        let error = Snapshot::verify_entries(
            entries,
            EXPECTED_ENTRIES,
            EXPECTED_SHA256.to_owned(),
            EXPECTED_NPM_INTEGRITY.to_owned(),
        )
        .unwrap_err();
        assert_eq!(error.context, "IconEntry.rtl");
    }

    #[test]
    fn rejects_independent_sha512_integrity_mismatch() {
        let error = verify_integrity(EXPECTED_SHA256, "sha512-invalid").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Provenance);
        assert_eq!(error.context, "archive.npm_integrity");
    }
}
