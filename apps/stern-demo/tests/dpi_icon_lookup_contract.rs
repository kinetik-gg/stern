//! Generated raster-icon lookup coverage for STERN-DPI-006.

#[allow(dead_code)]
mod phosphor_icons {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/icons/phosphor/phosphor_icons.rs"
    ));
}

use phosphor_icons::{ICON_ATLASES, ICON_ENTRIES, PhosphorIcon, icon_image};

#[test]
fn generated_lookup_selects_adequate_rasters_at_every_release_scale() {
    let icons = ICON_ENTRIES
        .iter()
        .filter(|entry| entry.logical_size == 16 && entry.physical_size == 16)
        .map(|entry| entry.icon)
        .collect::<Vec<PhosphorIcon>>();

    for icon in icons {
        for logical_size in [16, 24] {
            let logical_size_f32 = if logical_size == 16 { 16.0 } else { 24.0 };
            for (scale, dense_physical, standard_physical) in
                [(1.0, 16, 24), (1.25, 20, 30), (1.5, 24, 36), (2.0, 32, 48)]
            {
                let required_physical = if logical_size == 16 {
                    dense_physical
                } else {
                    standard_physical
                };
                let required_physical_f32 = f32::from(
                    u16::try_from(required_physical).expect("supported icon physical size"),
                );
                let image = icon_image(icon, logical_size_f32, scale);
                let entry = ICON_ENTRIES
                    .iter()
                    .find(|entry| entry.image == image)
                    .expect("generated lookup result must name a generated entry");
                let atlas = ICON_ATLASES
                    .iter()
                    .find(|atlas| atlas.image == entry.atlas)
                    .expect("generated entry must name a generated atlas");

                assert_eq!(entry.icon, icon);
                assert_eq!(entry.logical_size, logical_size);
                assert!(entry.physical_size >= required_physical);
                assert!(entry.source.width >= required_physical_f32);
                assert!(entry.source.height >= required_physical_f32);
                assert!(atlas.physical_size >= required_physical);
                assert_eq!(
                    atlas.bytes.len(),
                    usize::try_from(atlas.width).expect("atlas width")
                        * usize::try_from(atlas.height).expect("atlas height")
                        * 4
                );
            }
        }
    }
}
