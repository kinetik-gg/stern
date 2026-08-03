//! Deterministic, non-disclosing repository identity conformance scanner.

mod args;
mod collation;
mod inventory;
mod marker;
mod mask;
mod output;
mod scan;
mod self_test;

#[cfg(test)]
mod test_support;

use std::env;
use std::io;
use std::path::Path;

use args::ParsedArgs;
use output::ProcessOutput;

/// Runs the scanner with the current process arguments and writes its JSON result.
///
/// The returned value is the process exit code: zero for success, one for an
/// identity violation, and two for invalid input or an execution error.
#[must_use]
pub fn run_from_env() -> i32 {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            let output = ProcessOutput::error(&error.to_string());
            output.write_to(&mut io::stdout().lock(), &mut io::stderr().lock());
            return output.exit_code;
        }
    };
    let output = execute(&args, &cwd);
    output.write_to(&mut io::stdout().lock(), &mut io::stderr().lock());
    output.exit_code
}

fn execute(arguments: &[String], cwd: &Path) -> ProcessOutput {
    match execute_checked(arguments, cwd) {
        Ok(output) => output,
        Err(error) => ProcessOutput::error(&error),
    }
}

fn execute_checked(arguments: &[String], cwd: &Path) -> Result<ProcessOutput, String> {
    let args = ParsedArgs::parse(arguments)?;
    let root = inventory::real_root(cwd, args.root.as_deref().unwrap_or("."))?;

    if args.self_test {
        self_test::run(&root)?;
        return Ok(ProcessOutput::self_test_success());
    }

    let configured = mask::ConfiguredRepository::parse(args.configured_repository.as_deref())?;
    let result = match args.scope.as_deref() {
        Some("tracked") => {
            let entries = inventory::effective_tracked_files(&root)?;
            scan::scan_files(&root, &entries, &configured, "tracked")?
        }
        Some("filesystem") => {
            let requested = args
                .path
                .as_deref()
                .ok_or_else(|| "filesystem scope requires path".to_owned())?;
            let start = inventory::resolve_inside_root(&root, requested)?;
            let entries = inventory::filesystem_entries(&root, &start)?;
            scan::scan_files(&root, &entries, &configured, "filesystem")?
        }
        Some("metadata") => scan::scan_metadata(&args.metadata, &configured)?,
        _ => return Err("scope must be tracked, filesystem, or metadata".to_owned()),
    };

    Ok(match result.failure {
        Some(failure) => ProcessOutput::violation(&failure),
        None => ProcessOutput::scan_success(
            args.scope.as_deref().expect("validated scope"),
            result.text_file_count,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_argument_is_an_error_record() {
        let output = execute(&["--unknown".to_owned()], Path::new("."));
        assert_eq!(output.exit_code, 2);
        assert_eq!(
            output.stderr.as_deref(),
            Some("{\"status\":\"error\",\"message\":\"unknown argument\"}")
        );
        assert!(output.stdout.is_none());
    }
}
