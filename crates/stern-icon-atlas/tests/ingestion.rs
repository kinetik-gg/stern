//! End-to-end pinned-source and normalization acceptance tests.

use std::{path::PathBuf, sync::OnceLock};

use stern_icon_atlas::{
    Catalog, CatalogRecord, Discovery, ErrorKind, FillRule, PathCommand, RtlMetadata, Snapshot,
    StrokeCap, StrokeJoin, Weight, assign_constant_names, assign_stable_ids,
    assign_stable_ids_with, constant_name, normalize_svg,
};

fn snapshot() -> &'static Snapshot {
    static SNAPSHOT: OnceLock<Snapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| Snapshot::open(archive_path()).expect("pinned snapshot must verify"))
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| Catalog::from_snapshot(snapshot()).expect("catalog must parse"))
}

fn discovery() -> &'static Discovery {
    static DISCOVERY: OnceLock<Discovery> = OnceLock::new();
    DISCOVERY.get_or_init(|| {
        Discovery::from_snapshot(snapshot(), catalog()).expect("assets must discover")
    })
}

fn archive_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../third-party/phosphor/phosphor-core-2.1.1.tgz")
}

#[test]
fn verifies_exact_published_provenance_and_schema_absence() {
    let provenance = snapshot().provenance();
    assert_eq!(provenance.package, "@phosphor-icons/core");
    assert_eq!(provenance.version, "2.1.1");
    assert_eq!(
        provenance.source_url,
        "https://registry.npmjs.org/@phosphor-icons/core/-/core-2.1.1.tgz"
    );
    assert_eq!(
        provenance.sha256,
        "313332be6190b724da24107addd781799b48bf76b13963f24501112ffe1baadd"
    );
    assert_eq!(
        provenance.npm_integrity,
        "sha512-v4ARvrip4qBCImOE5rmPUylOEK4iiED9ZyKjcvzuezqMaiRASCHKcRIuvvxL/twvLpkfnEODCOJp5dM4eZilxQ=="
    );
    assert_eq!(provenance.archive_entries, 9_081);
    assert_eq!(provenance.license, "MIT");
    assert!(!provenance.has_rtl_metadata);
    assert!(
        catalog()
            .records
            .iter()
            .all(|record| record.rtl == RtlMetadata::AbsentInUpstreamSchema)
    );
}

#[test]
fn corrupt_or_missing_sources_fail_deterministically() {
    let mut bytes = std::fs::read(archive_path()).unwrap();
    bytes[100] ^= 1;
    let first = Snapshot::from_bytes(&bytes).unwrap_err();
    let second = Snapshot::from_bytes(&bytes).unwrap_err();
    assert_eq!(first, second);
    assert_eq!(first.kind, ErrorKind::Provenance);
    assert_eq!(first.context, "archive.sha256");

    let missing = Snapshot::open(archive_path().with_extension("missing")).unwrap_err();
    assert_eq!(missing.kind, ErrorKind::Io);
}

#[test]
fn parses_complete_catalog_and_deprecated_aliases() {
    assert_eq!(catalog().records.len(), 1_512);
    assert_eq!(catalog().alias_count(), 18);
    let acorn = &catalog().records[0];
    assert_eq!(acorn.name, "acorn");
    assert_eq!(acorn.pascal_name, "Acorn");
    assert_eq!(acorn.codepoint, 60_314);
    assert_eq!(acorn.published_in, "2.1");
    let asclepius = catalog()
        .records
        .iter()
        .find(|record| record.name == "asclepius")
        .unwrap();
    assert_eq!(asclepius.alias.as_ref().unwrap().name, "caduceus");
}

#[test]
fn malformed_catalog_records_and_aliases_fail() {
    let source = snapshot().text("package/dist/icons.d.ts").unwrap();
    let cases = [
        source.replacen("readonly codepoint: 60314;", "readonly codepoint: nope;", 1),
        source.replacen(
            "readonly pascal_name: \"Acorn\";",
            "readonly name: \"duplicate\";\n    readonly pascal_name: \"Acorn\";",
            1,
        ),
        source.replacen(
            "readonly pascal_name: \"Acorn\";",
            "readonly mystery: \"unknown\";\n    readonly pascal_name: \"Acorn\";",
            1,
        ),
        source.replacen(
            "readonly updated_in: 2.1;",
            "readonly updated_in: 2.1; readonly trailing: 1;",
            1,
        ),
        source.replacen(
            "readonly pascal_name: \"Acorn\";",
            "readonly pascal_name: \"acorn\";",
            1,
        ),
        source.replacen(
            "readonly pascal_name: \"Acorn\";",
            "readonly pascal_name: \"Wrong\";",
            1,
        ),
        source.replacen(
            "readonly pascal_name: \"Caduceus\";",
            "readonly pascal_name: \"caduceus\";",
            1,
        ),
        source.replacen("IconCategory.FINANCE", "IconCategory.", 1),
        source.replacen("IconCategory.FINANCE", "IconCategory.UNKNOWN", 1),
        source.replacen(
            "readonly published_in: 2.1;",
            "readonly published_in: 2.1.0;",
            1,
        ),
        source.replacen(
            "readonly name: \"caduceus\";",
            "readonly obsolete_name: \"caduceus\";",
            1,
        ),
        format!("{source}\nreadonly unexpected: true;"),
    ];
    for malformed in cases {
        assert_eq!(
            Catalog::parse(&malformed).unwrap_err().kind,
            ErrorKind::Catalog
        );
    }
}

#[test]
fn catalog_requires_the_exact_official_preamble() {
    let source = snapshot()
        .text("package/dist/icons.d.ts")
        .unwrap()
        .replace("\r\n", "\n");
    let import = "import { IconCategory, FigmaCategory } from \"./types\";\n";
    let declaration = "export type PhosphorIcon = (typeof icons)[number];\n";
    assert!(Catalog::parse(&source.replace('\n', "\r\n")).is_ok());

    let cases = [
        source.replacen(import, "", 1),
        source.replacen(declaration, "", 1),
        source.replacen(import, &format!("{import}{import}"), 1),
        source.replacen(declaration, &format!("{declaration}{declaration}"), 1),
        source.replacen(
            &format!("{import}{declaration}"),
            &format!("{declaration}{import}"),
            1,
        ),
        format!("unknown leading text\n{source}"),
        source.replacen(declaration, &format!("{declaration}\n"), 1),
        source.replacen(
            declaration,
            &format!("{declaration}declare const unknown: true;\n"),
            1,
        ),
        source.replacen('\n', "\r", 1),
    ];
    for malformed in cases {
        assert_eq!(
            Catalog::parse(&malformed).unwrap_err().kind,
            ErrorKind::Catalog
        );
    }
}

#[test]
fn catalog_string_literals_preserve_supported_escapes_and_reject_malformed_data() {
    let source = snapshot().text("package/dist/icons.d.ts").unwrap();
    let escaped = source.replacen(
        "\"savings\"",
        r#""quote:\" slash:\\ line:\n tab:\t return:\r""#,
        1,
    );
    let parsed = Catalog::parse(&escaped).unwrap();
    assert_eq!(
        parsed.records[0].tags[1],
        "quote:\" slash:\\ line:\n tab:\t return:\r"
    );

    let malformed_values = [
        r#""bad\q""#.to_owned(),
        r#""bad\x41""#.to_owned(),
        r#""bad\u0041""#.to_owned(),
        r#""dangling\"#.to_owned(),
        "\"raw\nnewline\"".to_owned(),
        "\"raw\ttab\"".to_owned(),
        format!("\"raw{}control\"", '\u{1}'),
    ];
    for value in malformed_values {
        let malformed = source.replacen("\"savings\"", &value, 1);
        assert_eq!(
            Catalog::parse(&malformed).unwrap_err().kind,
            ErrorKind::Catalog
        );
    }
    let malformed_name = source.replacen("\"acorn\"", r#""a\q""#, 1);
    assert_eq!(
        Catalog::parse(&malformed_name).unwrap_err().kind,
        ErrorKind::Catalog
    );
}

#[test]
fn discovers_every_canonical_icon_in_all_six_weights() {
    assert_eq!(discovery().icons.len(), 1_512);
    assert_eq!(discovery().asset_count(), 9_072);
    for weight in Weight::ALL {
        assert_eq!(
            discovery()
                .assets()
                .filter(|asset| asset.weight == weight)
                .count(),
            1_512
        );
    }
    let acorn = &discovery().icons[0];
    assert_eq!(
        acorn
            .assets
            .iter()
            .map(|asset| asset.weight)
            .collect::<Vec<_>>(),
        Weight::ALL
    );
    assert_eq!(
        acorn.assets[2].archive_path,
        "package/assets/regular/acorn.svg"
    );
    assert_eq!(
        acorn.assets[5].archive_path,
        "package/assets/duotone/acorn-duotone.svg"
    );
}

#[test]
fn discovery_rejects_missing_extra_and_misnamed_assets() {
    let record = CatalogRecord {
        name: "sample".to_owned(),
        pascal_name: "Sample".to_owned(),
        alias: None,
        categories: vec!["SYSTEM".to_owned()],
        figma_category: "SYSTEM".to_owned(),
        tags: Vec::new(),
        codepoint: 1,
        published_in: "1".to_owned(),
        updated_in: "1".to_owned(),
        rtl: RtlMetadata::AbsentInUpstreamSchema,
    };
    let tiny = Catalog {
        records: vec![record],
    };
    let missing = [
        "package/assets/thin/sample-thin.svg",
        "package/assets/light/sample-light.svg",
        "package/assets/regular/sample.svg",
        "package/assets/bold/sample-bold.svg",
        "package/assets/fill/sample-fill.svg",
    ];
    assert_eq!(
        Discovery::from_paths(&tiny, missing).unwrap_err().kind,
        ErrorKind::Discovery
    );
    let extra = ["package/assets/mystery/sample.svg"];
    assert_eq!(
        Discovery::from_paths(&tiny, extra).unwrap_err().kind,
        ErrorKind::Discovery
    );
    let misnamed = ["package/assets/thin/sample.svg"];
    assert_eq!(
        Discovery::from_paths(&tiny, misnamed).unwrap_err().kind,
        ErrorKind::Discovery
    );
    let nested = ["package/assets/regular/nested/sample.svg"];
    assert_eq!(
        Discovery::from_paths(&tiny, nested).unwrap_err().kind,
        ErrorKind::Discovery
    );
    let structurally_extra = ["package/assets/regular.svg"];
    assert_eq!(
        Discovery::from_paths(&tiny, structurally_extra)
            .unwrap_err()
            .kind,
        ErrorKind::Discovery
    );
}

#[test]
fn normalizes_all_nine_thousand_seventy_two_assets_offline() {
    let mut paths = 0_usize;
    for asset in discovery().assets() {
        let source = snapshot().text(&asset.archive_path).unwrap();
        let normalized =
            normalize_svg(&asset.archive_path, source).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!((normalized.width, normalized.height), (256.0, 256.0));
        assert!(!normalized.paths.is_empty());
        paths += normalized.paths.len();
    }
    assert_eq!(paths, 10_592);
}

#[test]
fn preserves_representative_weight_and_duotone_layers() {
    let acorn = discovery()
        .icons
        .iter()
        .find(|icon| icon.name == "acorn")
        .unwrap();
    for asset in &acorn.assets {
        let icon = normalize_svg(
            &asset.archive_path,
            snapshot().text(&asset.archive_path).unwrap(),
        )
        .unwrap();
        assert!(!icon.paths.is_empty(), "{}", asset.weight);
    }
    let duotone = &acorn.assets[5];
    let icon = normalize_svg(
        &duotone.archive_path,
        snapshot().text(&duotone.archive_path).unwrap(),
    )
    .unwrap();
    assert_eq!(icon.paths.len(), 2);
    assert!((icon.paths[0].opacity - 0.2).abs() < f64::EPSILON);
    assert!((icon.paths[1].opacity - 1.0).abs() < f64::EPSILON);
    let fill = &acorn.assets[4];
    assert!(
        normalize_svg(
            &fill.archive_path,
            snapshot().text(&fill.archive_path).unwrap()
        )
        .unwrap()
        .paths
        .iter()
        .all(|path| path.filled)
    );
}

#[test]
fn synthetic_svg_preserves_subpaths_arcs_fill_and_stroke_styles() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="none"><path d="M10 10h20v20z M50 50A20 10 30 0 1 100 100q10 10 20 0t20 0" fill="currentColor" fill-rule="evenodd" opacity="0.25" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="bevel"/></svg>"#;
    let icon = normalize_svg("synthetic", svg).unwrap();
    let path = &icon.paths[0];
    assert_eq!(path.fill_rule, FillRule::EvenOdd);
    assert!((path.opacity - 0.25).abs() < f64::EPSILON);
    assert_eq!(
        path.commands
            .iter()
            .filter(|command| matches!(command, PathCommand::MoveTo(_)))
            .count(),
        2
    );
    assert!(
        path.commands
            .iter()
            .any(|command| matches!(command, PathCommand::CubicTo { .. }))
    );
    assert!(
        path.commands
            .iter()
            .any(|command| matches!(command, PathCommand::QuadTo { .. }))
    );
    let stroke = path.stroke.unwrap();
    assert!((stroke.width - 3.0).abs() < f64::EPSILON);
    assert_eq!(stroke.cap, StrokeCap::Round);
    assert_eq!(stroke.join, StrokeJoin::Bevel);
}

#[test]
fn svg_validation_rejects_invalid_documents_and_data() {
    let cases = [
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0L1 1"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="1"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path d="M nope"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path d="M0 0L1 1" opacity="2"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path d="M0 0"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">text<path d="M0 0L1 1"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path d="M0 0L1 1">text</path></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path d="M0 0L1 1"><!--content--></path></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><svg><path d="M0 0L1 1"/></svg></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path d="M0 0L1 1"><path d="M0 0L1 1"/></path></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path d="M0 0A-1 2 0 0 1 4 4"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path d="M1e308 0c1e308 0 0 0 0 1"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path d="M1e308 0l1e308 0"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path d="M0 0A1 1 1e309 0 1 4 4"/></svg>"#,
    ];
    for source in cases {
        assert_eq!(
            normalize_svg("invalid", source).unwrap_err().kind,
            ErrorKind::Svg
        );
    }
}

#[test]
fn constant_names_are_valid_deterministic_and_collision_safe() {
    assert_eq!(constant_name("floppy-disk"), "FLOPPY_DISK");
    assert_eq!(constant_name("3d-rotate"), "ICON_3D_ROTATE");
    assert_eq!(constant_name("type"), "TYPE_ICON");
    assert_eq!(constant_name("odd.name+value"), "ODD_NAME_VALUE");
    assert_eq!(
        assign_constant_names(["b-icon", "a-icon"]).unwrap(),
        assign_constant_names(["b-icon", "a-icon"]).unwrap()
    );
    assert_eq!(
        assign_constant_names(["foo-bar", "foo_bar"])
            .unwrap_err()
            .kind,
        ErrorKind::NameCollision
    );
    let names = catalog().records.iter().flat_map(|record| {
        std::iter::once(record.name.as_str())
            .chain(record.alias.iter().map(|alias| alias.name.as_str()))
    });
    assert_eq!(assign_constant_names(names).unwrap().len(), 1_530);
}

#[test]
fn stable_ids_are_repeatable_weight_specific_and_collision_safe() {
    let first = assign_stable_ids([("acorn", Weight::Regular), ("acorn", Weight::Fill)]).unwrap();
    let second = assign_stable_ids([("acorn", Weight::Regular), ("acorn", Weight::Fill)]).unwrap();
    assert_eq!(first, second);
    assert_ne!(
        first[&("acorn".to_owned(), Weight::Regular)],
        first[&("acorn".to_owned(), Weight::Fill)]
    );
    assert_eq!(
        assign_stable_ids_with([("a", Weight::Regular), ("b", Weight::Regular)], |_| 7)
            .unwrap_err()
            .kind,
        ErrorKind::IdCollision
    );
}

#[test]
fn repeated_discovery_and_normalization_are_identical() {
    let next_catalog = Catalog::from_snapshot(snapshot()).unwrap();
    let next_discovery = Discovery::from_snapshot(snapshot(), &next_catalog).unwrap();
    assert_eq!(&next_catalog, catalog());
    assert_eq!(&next_discovery, discovery());
    let asset = &discovery().icons[500].assets[5];
    let source = snapshot().text(&asset.archive_path).unwrap();
    assert_eq!(
        normalize_svg(&asset.archive_path, source).unwrap(),
        normalize_svg(&asset.archive_path, source).unwrap()
    );
}
