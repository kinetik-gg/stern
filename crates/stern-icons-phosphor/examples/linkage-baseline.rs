//! Release fixture retaining exactly one generated icon definition.

use std::hint::black_box;

use stern_icons_phosphor::regular;

fn main() {
    let icon = black_box(regular::AIRPLANE);
    let graphic = black_box(icon.icon().graphic());
    let elements = graphic
        .layers
        .iter()
        .flat_map(|layer| layer.paths)
        .map(|path| path.elements.len())
        .sum::<usize>();
    println!("{}:{elements}", black_box(icon.identity()));
}
