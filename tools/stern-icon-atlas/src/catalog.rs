//! Parser for the official declaration-only Phosphor catalog.

use std::collections::BTreeSet;

use crate::{Error, ErrorKind, Result, Snapshot, assign_constant_names};

const CATALOG_PATH: &str = "package/dist/icons.d.ts";
const CATALOG_PREAMBLE: &str = "import { IconCategory, FigmaCategory } from \"./types\";\nexport type PhosphorIcon = (typeof icons)[number];\n";

/// Upstream RTL metadata state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RtlMetadata {
    /// The pinned upstream `IconEntry` schema declares no RTL field.
    AbsentInUpstreamSchema,
}

/// Deprecated upstream name for a canonical icon.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogAlias {
    /// Kebab-case alias.
    pub name: String,
    /// Upstream Pascal-case alias.
    pub pascal_name: String,
}

/// One canonical upstream catalog record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogRecord {
    /// Canonical kebab-case name.
    pub name: String,
    /// Upstream Pascal-case name.
    pub pascal_name: String,
    /// Optional deprecated alias.
    pub alias: Option<CatalogAlias>,
    /// Ordered upstream categories.
    pub categories: Vec<String>,
    /// Upstream Figma category.
    pub figma_category: String,
    /// Ordered upstream search tags.
    pub tags: Vec<String>,
    /// Font codepoint retained as metadata.
    pub codepoint: u32,
    /// Upstream publication version spelling.
    pub published_in: String,
    /// Upstream update version spelling.
    pub updated_in: String,
    /// Honest RTL metadata state; never inferred from geometry or name.
    pub rtl: RtlMetadata,
}

/// Deterministically ordered canonical catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Catalog {
    /// Canonical records in upstream declaration order.
    pub records: Vec<CatalogRecord>,
}

impl Catalog {
    /// Parses the declaration catalog from a verified snapshot without executing it.
    ///
    /// # Errors
    ///
    /// Returns a catalog or archive error if metadata is missing or malformed.
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self> {
        Self::parse(snapshot.text(CATALOG_PATH)?)
    }

    /// Parses official `icons.d.ts` declaration text.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Catalog`] for a malformed record, alias, or tuple.
    pub fn parse(text: &str) -> Result<Self> {
        let normalized = text.replace("\r\n", "\n");
        if normalized.contains('\r') {
            return Err(Error::new(
                ErrorKind::Catalog,
                CATALOG_PATH,
                "catalog contains an unsupported bare carriage return",
            ));
        }
        let declaration = normalized.strip_prefix(CATALOG_PREAMBLE).ok_or_else(|| {
            Error::new(
                ErrorKind::Catalog,
                CATALOG_PATH,
                "catalog preamble must contain exactly the official import and type declaration",
            )
        })?;
        let prefix = "export declare const icons: readonly [{";
        if declaration.matches(prefix).count() != 1 {
            return Err(Error::new(
                ErrorKind::Catalog,
                CATALOG_PATH,
                "expected exactly one icons tuple declaration",
            ));
        }
        let body_with_envelope = declaration.strip_prefix(prefix).ok_or_else(|| {
            Error::new(
                ErrorKind::Catalog,
                CATALOG_PATH,
                "icons tuple declaration must immediately follow the official preamble",
            )
        })?;
        let end = body_with_envelope.rfind("}];").ok_or_else(|| {
            Error::new(
                ErrorKind::Catalog,
                CATALOG_PATH,
                "icons tuple declaration is unterminated",
            )
        })?;
        if !body_with_envelope[end + 3..].trim().is_empty() {
            return Err(Error::new(
                ErrorKind::Catalog,
                CATALOG_PATH,
                "unknown trailing catalog declaration data",
            ));
        }
        let body = &body_with_envelope[..end];
        let mut records = Vec::new();
        for (index, raw) in body.split("}, {").enumerate() {
            records.push(parse_record(raw.trim(), index)?);
        }
        if records.is_empty() {
            return Err(Error::new(
                ErrorKind::Catalog,
                CATALOG_PATH,
                "catalog is empty",
            ));
        }
        validate_records(&records)?;
        Ok(Self { records })
    }

    /// Number of deprecated aliases.
    #[must_use]
    pub fn alias_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.alias.is_some())
            .count()
    }
}

fn parse_record(raw: &str, index: usize) -> Result<CatalogRecord> {
    let context = format!("{CATALOG_PATH} record {index}");
    let mut cursor = RecordCursor::new(raw, &context);
    let name = cursor.string("name")?;
    let pascal_name = cursor.string("pascal_name")?;
    let alias = cursor.alias()?;
    let categories = cursor.enums("categories", "IconCategory.")?;
    if categories.is_empty() {
        return Err(Error::new(
            ErrorKind::Catalog,
            &context,
            "categories cannot be empty",
        ));
    }
    let figma_category = cursor.enumeration("figma_category", "FigmaCategory.")?;
    let tags = cursor.strings("tags")?;
    let codepoint = cursor.unsigned("codepoint")?;
    let published_in = cursor.version("published_in")?;
    let updated_in = cursor.version("updated_in")?;
    cursor.finish()?;
    Ok(CatalogRecord {
        name,
        pascal_name,
        alias,
        categories,
        figma_category,
        tags,
        codepoint,
        published_in,
        updated_in,
        rtl: RtlMetadata::AbsentInUpstreamSchema,
    })
}

struct RecordCursor<'a> {
    rest: &'a str,
    context: &'a str,
}

impl<'a> RecordCursor<'a> {
    fn new(rest: &'a str, context: &'a str) -> Self {
        Self {
            rest: rest.trim(),
            context,
        }
    }

    fn string(&mut self, field: &str) -> Result<String> {
        self.prefix(&format!("readonly {field}:"))?;
        let (value, consumed) = parse_quoted(self.rest, self.context)?;
        self.rest = self.rest[consumed..].trim_start();
        self.prefix(";")?;
        Ok(value)
    }

    fn alias(&mut self) -> Result<Option<CatalogAlias>> {
        if !self.rest.starts_with("readonly alias:") {
            return Ok(None);
        }
        self.prefix("readonly alias:")?;
        self.prefix("{")?;
        let name = self.string("name")?;
        let pascal_name = self.string("pascal_name")?;
        self.prefix("};")?;
        Ok(Some(CatalogAlias { name, pascal_name }))
    }

    fn enums(&mut self, field: &str, enum_prefix: &str) -> Result<Vec<String>> {
        let body = self.array(field)?;
        if body.trim().is_empty() {
            return Ok(Vec::new());
        }
        body.split(',')
            .map(|item| parse_enum_token(item.trim(), enum_prefix, self.context, field))
            .collect()
    }

    fn enumeration(&mut self, field: &str, enum_prefix: &str) -> Result<String> {
        self.prefix(&format!("readonly {field}:"))?;
        let end = self.rest.find(';').ok_or_else(|| {
            Error::new(
                ErrorKind::Catalog,
                self.context,
                format!("field `{field}` is unterminated"),
            )
        })?;
        let value = parse_enum_token(self.rest[..end].trim(), enum_prefix, self.context, field)?;
        self.rest = self.rest[end + 1..].trim_start();
        Ok(value)
    }

    fn strings(&mut self, field: &str) -> Result<Vec<String>> {
        let mut rest = self.array(field)?.trim();
        let mut values = Vec::new();
        while !rest.is_empty() {
            let (value, consumed) = parse_quoted(rest, self.context)?;
            values.push(value);
            rest = rest[consumed..].trim_start();
            if rest.is_empty() {
                break;
            }
            rest = rest
                .strip_prefix(',')
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Catalog,
                        self.context,
                        format!("field `{field}` has an invalid separator"),
                    )
                })?
                .trim_start();
            if rest.is_empty() {
                return Err(Error::new(
                    ErrorKind::Catalog,
                    self.context,
                    format!("field `{field}` has a trailing separator"),
                ));
            }
        }
        Ok(values)
    }

    fn unsigned(&mut self, field: &str) -> Result<u32> {
        let value = self.scalar(field)?;
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(Error::new(
                ErrorKind::Catalog,
                self.context,
                format!("field `{field}` is not an unsigned integer"),
            ));
        }
        value.parse().map_err(|_| {
            Error::new(
                ErrorKind::Catalog,
                self.context,
                format!("field `{field}` overflows u32"),
            )
        })
    }

    fn version(&mut self, field: &str) -> Result<String> {
        let value = self.scalar(field)?;
        let mut parts = value.split('.');
        let major = parts.next().unwrap_or_default();
        let minor = parts.next();
        let valid = !major.is_empty()
            && major.bytes().all(|byte| byte.is_ascii_digit())
            && minor.is_none_or(|part| {
                !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
            })
            && parts.next().is_none();
        if !valid {
            return Err(Error::new(
                ErrorKind::Catalog,
                self.context,
                format!("field `{field}` is not a strict numeric version"),
            ));
        }
        Ok(value.to_owned())
    }

    fn scalar(&mut self, field: &str) -> Result<&'a str> {
        self.prefix(&format!("readonly {field}:"))?;
        let end = self.rest.find(';').ok_or_else(|| {
            Error::new(
                ErrorKind::Catalog,
                self.context,
                format!("field `{field}` is unterminated"),
            )
        })?;
        let value = self.rest[..end].trim();
        self.rest = self.rest[end + 1..].trim_start();
        Ok(value)
    }

    fn array(&mut self, field: &str) -> Result<&'a str> {
        self.prefix(&format!("readonly {field}:"))?;
        self.prefix("readonly [")?;
        let end = self.rest.find("];").ok_or_else(|| {
            Error::new(
                ErrorKind::Catalog,
                self.context,
                format!("field `{field}` is unterminated"),
            )
        })?;
        let body = &self.rest[..end];
        self.rest = self.rest[end + 2..].trim_start();
        Ok(body)
    }

    fn prefix(&mut self, expected: &str) -> Result<()> {
        self.rest = self
            .rest
            .strip_prefix(expected)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::Catalog,
                    self.context,
                    format!("expected `{expected}`, found `{}`", preview(self.rest)),
                )
            })?
            .trim_start();
        Ok(())
    }

    fn finish(self) -> Result<()> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Catalog,
                self.context,
                format!("unknown or trailing record data `{}`", preview(self.rest)),
            ))
        }
    }
}

fn validate_records(records: &[CatalogRecord]) -> Result<()> {
    let mut canonical = BTreeSet::new();
    let mut all_names = Vec::new();
    for record in records {
        validate_name(&record.name, "canonical name")?;
        validate_pascal(&record.pascal_name, &record.name, "canonical Pascal name")?;
        if !canonical.insert(record.name.as_str()) {
            return Err(Error::new(
                ErrorKind::Catalog,
                &record.name,
                "duplicate canonical name",
            ));
        }
        all_names.push(record.name.as_str());
        if let Some(alias) = &record.alias {
            validate_name(&alias.name, "alias")?;
            validate_pascal(&alias.pascal_name, &alias.name, "alias Pascal name")?;
            all_names.push(alias.name.as_str());
        }
    }
    assign_constant_names(all_names)?;
    Ok(())
}

fn validate_name(name: &str, kind: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--");
    if valid {
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::Catalog,
            name,
            format!("{kind} is not canonical kebab case"),
        ))
    }
}

fn validate_pascal(value: &str, context: &str, kind: &str) -> Result<()> {
    let expected = context
        .split('-')
        .map(|part| {
            let mut characters = part.chars();
            characters.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + characters.as_str()
            })
        })
        .collect::<String>();
    let valid = value == expected;
    if valid {
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::Catalog,
            context,
            format!("{kind} must be exactly `{expected}`, found `{value}`"),
        ))
    }
}

fn parse_enum_token(value: &str, prefix: &str, context: &str, field: &str) -> Result<String> {
    let token = value.strip_prefix(prefix).ok_or_else(|| {
        Error::new(
            ErrorKind::Catalog,
            context,
            format!("field `{field}` has the wrong enum type"),
        )
    })?;
    if token.is_empty()
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(Error::new(
            ErrorKind::Catalog,
            context,
            format!("field `{field}` contains an invalid enum token"),
        ));
    }
    let known = match prefix {
        "IconCategory." => matches!(
            token,
            "ARROWS"
                | "BRAND"
                | "COMMERCE"
                | "COMMUNICATION"
                | "DESIGN"
                | "DEVELOPMENT"
                | "EDITOR"
                | "FINANCE"
                | "GAMES"
                | "HEALTH"
                | "MAP"
                | "MEDIA"
                | "NATURE"
                | "OBJECTS"
                | "OFFICE"
                | "PEOPLE"
                | "SYSTEM"
                | "WEATHER"
        ),
        "FigmaCategory." => matches!(
            token,
            "ARROWS"
                | "BRAND"
                | "COMMERCE"
                | "COMMUNICATION"
                | "DESIGN"
                | "DEVELOPMENT"
                | "EDUCATION"
                | "FINANCE"
                | "GAMES"
                | "HEALTH"
                | "MAP"
                | "MEDIA"
                | "OFFICE"
                | "PEOPLE"
                | "SECURITY"
                | "SYSTEM"
                | "TIME"
                | "WEATHER"
        ),
        _ => false,
    };
    if !known {
        return Err(Error::new(
            ErrorKind::Catalog,
            context,
            format!("field `{field}` contains unknown enum token `{token}`"),
        ));
    }
    Ok(token.to_owned())
}

fn preview(value: &str) -> String {
    value.chars().take(40).collect()
}

fn parse_quoted(text: &str, context: &str) -> Result<(String, usize)> {
    if !text.starts_with('"') {
        return Err(Error::new(
            ErrorKind::Catalog,
            context,
            "expected a quoted string",
        ));
    }
    let mut value = String::new();
    let mut escaped = false;
    for (offset, character) in text[1..].char_indices() {
        if escaped {
            let decoded = match character {
                '"' => '"',
                '\\' => '\\',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => {
                    return Err(Error::new(
                        ErrorKind::Catalog,
                        context,
                        format!("unsupported string escape `\\{other}`"),
                    ));
                }
            };
            value.push(decoded);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Ok((value, offset + 2));
        } else if character.is_control() {
            return Err(Error::new(
                ErrorKind::Catalog,
                context,
                "quoted string contains an unescaped control character",
            ));
        } else {
            value.push(character);
        }
    }
    if escaped {
        return Err(Error::new(
            ErrorKind::Catalog,
            context,
            "quoted string ends with a dangling escape",
        ));
    }
    Err(Error::new(
        ErrorKind::Catalog,
        context,
        "unterminated quoted string",
    ))
}
