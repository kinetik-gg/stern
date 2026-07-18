//! Command-line entry point for the Stern identity scanner.

fn main() {
    std::process::exit(stern_identity_scan::run_from_env());
}
