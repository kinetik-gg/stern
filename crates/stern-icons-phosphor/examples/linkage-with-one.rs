//! Release fixture retaining the baseline plus exactly one definition.

use std::hint::black_box;

use stern_icons_phosphor::regular;

fn measure(icon: stern_icons_phosphor::PhosphorIcon) -> usize {
    black_box(icon.icon().graphic())
        .layers
        .iter()
        .flat_map(|layer| layer.paths)
        .map(|path| path.elements.len())
        .sum()
}

fn main() {
    let airplane = black_box(regular::AIRPLANE);
    let floppy = black_box(regular::FLOPPY_DISK);
    println!(
        "{}:{}:{}:{}",
        black_box(airplane.identity()),
        measure(airplane),
        black_box(floppy.identity()),
        measure(floppy)
    );
}
