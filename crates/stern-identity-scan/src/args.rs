#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct ParsedArgs {
    pub(crate) self_test: bool,
    pub(crate) root: Option<String>,
    pub(crate) scope: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) metadata: Vec<String>,
    pub(crate) configured_repository: Option<String>,
}

impl ParsedArgs {
    pub(crate) fn parse(arguments: &[String]) -> Result<Self, String> {
        let mut parsed = Self::default();
        let mut index = 0;
        while index < arguments.len() {
            let argument = &arguments[index];
            if argument == "--self-test" {
                parsed.self_test = true;
                index += 1;
                continue;
            }
            if !matches!(
                argument.as_str(),
                "--root" | "--scope" | "--path" | "--metadata" | "--configured-repository"
            ) {
                return Err("unknown argument".to_owned());
            }
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| "missing argument value".to_owned())?
                .clone();
            match argument.as_str() {
                "--root" => parsed.root = Some(value),
                "--scope" => parsed.scope = Some(value),
                "--path" => parsed.path = Some(value),
                "--metadata" => parsed.metadata.push(value),
                "--configured-repository" => parsed.configured_repository = Some(value),
                _ => unreachable!("argument was validated"),
            }
            index += 2;
        }
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn defaults_match_the_original_parser() {
        assert_eq!(ParsedArgs::parse(&[]).unwrap(), ParsedArgs::default());
    }

    #[test]
    fn values_overwrite_and_metadata_repeats() {
        let parsed = ParsedArgs::parse(&strings(&[
            "--root",
            "first",
            "--root",
            "second",
            "--metadata",
            "a=1",
            "--self-test",
            "--metadata",
            "b=2",
            "--configured-repository",
            "owner/stern",
        ]))
        .unwrap();
        assert!(parsed.self_test);
        assert_eq!(parsed.root.as_deref(), Some("second"));
        assert_eq!(parsed.metadata, ["a=1", "b=2"]);
        assert_eq!(parsed.configured_repository.as_deref(), Some("owner/stern"));
    }

    #[test]
    fn rejects_unknown_and_missing_values() {
        assert_eq!(
            ParsedArgs::parse(&strings(&["--wat"])),
            Err("unknown argument".to_owned())
        );
        assert_eq!(
            ParsedArgs::parse(&strings(&["--root"])),
            Err("missing argument value".to_owned())
        );
    }

    #[test]
    fn option_spelling_can_be_consumed_as_a_value() {
        let parsed = ParsedArgs::parse(&strings(&["--root", "--scope"])).unwrap();
        assert_eq!(parsed.root.as_deref(), Some("--scope"));
        assert!(parsed.scope.is_none());
    }
}
