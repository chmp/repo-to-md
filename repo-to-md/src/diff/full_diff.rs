use anyhow::Result;
use anyhow::bail;

use super::DiffFile;
use super::DiffFileParser;
use super::MultilineParser;

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct Diff<'a> {
    pub files: Vec<DiffFile<'a>>,
}

impl<'a> Diff<'a> {
    pub fn into_static(self) -> Diff<'static> {
        Diff {
            files: self.files.into_iter().map(DiffFile::into_static).collect(),
        }
    }
}

pub struct DiffParser;

impl<'a> MultilineParser<'a> for DiffParser {
    const NAME: &'static str = "diff";

    type Output = Diff<'a>;

    fn parse_lines(
        &self,
        lines: &'a [&'a str],
    ) -> anyhow::Result<Option<(Self::Output, &'a [&'a str])>> {
        let (files, rest) = DiffFileParser.parse_lines_many(lines)?;

        let result = Diff { files };
        Ok(Some((result, rest)))
    }
}

pub fn parse<'a>(lines: &'a [&'a str]) -> Result<Diff<'a>> {
    let (result, rest) = DiffParser.parse_lines_required(lines)?;
    for line in rest {
        if !line.trim().is_empty() {
            bail!("trailing content in diff: {line:?}");
        }
    }
    Ok(result)
}

#[test]
fn parse_full_diff_with_multiple_files() {
    let lines = [
        "diff --git a/src/lib.rs b/src/lib.rs",
        "index 7626a52..16399c7 100644",
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
        "@@ -1,1 +1,1 @@",
        " line",
        "diff --git a/README.md b/README.md",
        "index 1111111..2222222 100644",
        "--- a/README.md",
        "+++ b/README.md",
        "@@ -1,1 +1,1 @@",
        "-old",
        "+new",
        "@@ -10,1 +10,2 @@",
        " context",
        "+added",
        "",
    ];

    let diff = parse(&lines).unwrap();
    assert_eq!(diff.files.len(), 2);
    assert_eq!(diff.files[0].chunks.len(), 1);
    assert_eq!(diff.files[1].chunks.len(), 2);

    let _static_diff: Diff<'static> = diff.into_static();
}

#[test]
fn parse_full_diff_rejects_trailing_content() {
    let lines = [
        "diff --git a/src/lib.rs b/src/lib.rs",
        "index 7626a52..16399c7 100644",
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
        "@@ -1,1 +1,1 @@",
        " line",
        "not part of the diff",
    ];

    let error = parse(&lines).unwrap_err();
    assert!(
        error.to_string().contains("trailing content in diff"),
        "{error}"
    );
}
