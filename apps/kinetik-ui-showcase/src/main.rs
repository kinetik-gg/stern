//! Kinetik UI showcase smoke entry point.

fn main() {
    for scenario in kinetik_ui_showcase::all_scenarios() {
        println!(
            "{}: {} primitives",
            scenario.name,
            scenario.primitives.len()
        );
    }
}
