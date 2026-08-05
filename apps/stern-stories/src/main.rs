//! Story harness CLI: `render`, `diff`, `bless`, and `run`.

use std::path::PathBuf;
use std::process::ExitCode;

use stern_stories::ops::{DiffStatus, bless_directories, diff_directories, render_stories};
use stern_stories::story::registry;

const USAGE: &str = "usage: stern-stories <command> [options]

commands:
  render [--filter S] [--out DIR]
      Headless CPU render of every matching story variant to PNGs plus a
      contact sheet and manifest. Default out: target/stern-stories/render
  diff [--filter S] [--current DIR] [--goldens DIR] [--out DIR]
      Compare a render directory against goldens; writes visual diff images.
      Defaults: current target/stern-stories/render, goldens
      apps/stern-stories/goldens, out target/stern-stories/diff
  bless [--filter S] [--current DIR] [--goldens DIR]
      Copy a HUMAN-REVIEWED render into the goldens directory. Never run
      automatically; review the PNGs first.
  run [--filter S]
      Interactive story browser window.
  list [--filter S]
      Print matching story ids.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let options = match parse_options(&args[1..]) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let result = match command.as_str() {
        "render" => render(&options),
        "diff" => diff(&options),
        "bless" => bless(&options),
        "run" => stern_stories::interactive::run_browser(&options.filter).map(|()| true),
        "list" => Ok(list(&options)),
        _ => Err(format!("unknown command {command:?}")),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

struct Options {
    filter: String,
    out: Option<PathBuf>,
    current: Option<PathBuf>,
    goldens: Option<PathBuf>,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        filter: String::new(),
        out: None,
        current: None,
        goldens: None,
    };
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        let mut value = |name: &str| {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{name} requires a value"))
        };
        match flag.as_str() {
            "--filter" => options.filter = value("--filter")?,
            "--out" => options.out = Some(PathBuf::from(value("--out")?)),
            "--current" => options.current = Some(PathBuf::from(value("--current")?)),
            "--goldens" => options.goldens = Some(PathBuf::from(value("--goldens")?)),
            _ => return Err(format!("unknown option {flag:?}")),
        }
    }
    Ok(options)
}

fn default_render_dir() -> PathBuf {
    PathBuf::from("target/stern-stories/render")
}

fn default_goldens_dir() -> PathBuf {
    PathBuf::from("apps/stern-stories/goldens")
}

fn render(options: &Options) -> Result<bool, String> {
    let out = options.out.clone().unwrap_or_else(default_render_dir);
    let report = render_stories(&registry(), &options.filter, &out)?;
    for file in &report.files {
        println!("rendered {}", out.join(file).display());
    }
    println!("rendered {} files into {}", report.files.len(), out.display());
    for (file, diagnostics) in &report.diagnostics {
        for diagnostic in diagnostics {
            println!("diagnostic {file}: {diagnostic}");
        }
    }
    Ok(true)
}

fn diff(options: &Options) -> Result<bool, String> {
    let current = options.current.clone().unwrap_or_else(default_render_dir);
    let goldens = options.goldens.clone().unwrap_or_else(default_goldens_dir);
    let out = options
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from("target/stern-stories/diff"));
    let statuses = diff_directories(&current, &goldens, &out, &options.filter)?;
    let mut clean = true;
    for status in &statuses {
        match status {
            DiffStatus::Match(file) => println!("match   {file}"),
            DiffStatus::Changed(file, diff_path) => {
                clean = false;
                match diff_path {
                    Some(path) => println!("CHANGED {file} -> {}", path.display()),
                    None => println!("CHANGED {file} (dimensions differ)"),
                }
            }
            DiffStatus::MissingGolden(file) => {
                clean = false;
                println!("NO GOLDEN {file} (render reviewed? bless to accept)");
            }
            DiffStatus::MissingCurrent(file) => {
                clean = false;
                println!("MISSING {file} (golden exists, render does not)");
            }
        }
    }
    if clean {
        println!("all {} comparisons match", statuses.len());
    }
    Ok(clean)
}

fn bless(options: &Options) -> Result<bool, String> {
    let current = options.current.clone().unwrap_or_else(default_render_dir);
    let goldens = options.goldens.clone().unwrap_or_else(default_goldens_dir);
    let blessed = bless_directories(&current, &goldens, &options.filter)?;
    for file in &blessed {
        println!("blessed {file}");
    }
    println!(
        "blessed {} files into {} (human review is your responsibility)",
        blessed.len(),
        goldens.display()
    );
    Ok(true)
}

fn list(options: &Options) -> bool {
    for story in registry() {
        if stern_stories::story_matches_filter(story.id, &options.filter) {
            println!("{}  {}", story.id, story.title);
        }
    }
    true
}
