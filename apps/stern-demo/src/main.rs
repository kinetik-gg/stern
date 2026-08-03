//! Stern integration-demo evidence entry point.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use stern::core::ScaleFactor;
use stern::platform_winit::{WinitInputAdapter, WinitPlatformRequests};
use stern_demo::{DEMO_TITLE, DemoApp, demo_context};

/// Flags recognized by the demo CLI, listed in the unknown-argument error.
const SUPPORTED_FLAGS: &str = "--dump-identity-evidence <directory>";

/// Rejects any argument the demo CLI does not recognize. The only
/// recognized shape is `--dump-identity-evidence <directory>`; anything else
/// (including the retired `--dump-review-artifacts`) is unknown.
fn validate_arguments(args: &[String]) -> Result<(), &str> {
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--dump-identity-evidence" => index += 2,
            other => return Err(other),
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if let Err(unknown) = validate_arguments(&args) {
        eprintln!("stern-demo: unrecognized argument `{unknown}`");
        eprintln!("Supported flags:");
        eprintln!("  {SUPPORTED_FLAGS}");
        std::process::exit(1);
    }
    let mut app = DemoApp::new();
    let mut platform_input = WinitInputAdapter::new(ScaleFactor::ONE);
    platform_input.set_window_focused(true);
    let input = platform_input.into_input();
    let input_event_count = input.events.len();
    let output = app.frame(demo_context(input));

    if let Some(index) = args
        .iter()
        .position(|argument| argument == "--dump-identity-evidence")
    {
        let directory = args.get(index + 1).ok_or("missing evidence directory")?;
        dump_identity_evidence(Path::new(directory), &app, &output, input_event_count)?;
        return Ok(());
    }

    println!(
        "{DEMO_TITLE}: {} primitives, {} semantic nodes",
        output.primitives.len(),
        output.semantics.nodes().len()
    );
    Ok(())
}

fn dump_identity_evidence(
    directory: &Path,
    app: &DemoApp,
    output: &stern::core::FrameOutput,
    input_event_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(directory)?;
    let resources = app.render_resources();
    let translation = stern::render_vello::translate_primitives(&output.primitives, &resources);
    let platform_requests = WinitPlatformRequests::from_frame_output(output);
    let translated_title = platform_requests
        .window_title()
        .ok_or("missing translated Winit window title")?;
    let accessibility =
        stern::platform_winit::WinitAccessibilityUpdate::from_frame_output(output, app.focused())
            .map_err(|error| std::io::Error::other(format!("invalid semantic output: {error:?}")))?;
    let mut evidence = String::new();
    writeln!(evidence, "title={DEMO_TITLE}")?;
    writeln!(evidence, "package=stern-demo")?;
    writeln!(evidence, "facade=stern")?;
    writeln!(evidence, "winit_input_events={input_event_count}")?;
    writeln!(evidence, "winit_window_title={translated_title}")?;
    writeln!(evidence, "primitives={}", output.primitives.len())?;
    writeln!(evidence, "semantics={}", output.semantics.nodes().len())?;
    writeln!(evidence, "vello_commands={}", translation.commands.len())?;
    writeln!(
        evidence,
        "accessibility_nodes={}",
        accessibility.snapshot.nodes.len()
    )?;
    fs::write(directory.join("identity.txt"), evidence)?;
    Ok(())
}
