use std::borrow::Cow;

use super::LineParser;

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct DiffLine<'a> {
    from_status: Vec<DiffLineStatus>,
    to_status: DiffLineStatus,
    content: Cow<'a, str>,
}

impl<'a> DiffLine<'a> {
    pub fn into_static(self) -> DiffLine<'static> {
        DiffLine {
            from_status: self.from_status,
            to_status: self.to_status,
            content: Cow::Owned(self.content.into_owned()),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, serde::Serialize)]
pub enum DiffLineStatus {
    Added,
    Removed,
    Unchanged,
}

pub struct DiffLineParser {
    pub number_of_parents: usize,
}

impl DiffLineParser {
    pub fn new(number_of_parents: usize) -> Self {
        Self { number_of_parents }
    }
}

impl<'a> LineParser<'a> for DiffLineParser {
    const NAME: &'static str = "diff line";

    type Output = DiffLine<'a>;

    fn parse_line(&self, line: &'a str) -> anyhow::Result<Option<Self::Output>> {
        if self.number_of_parents == 0 {
            return Ok(None);
        }

        let mut from_status = Vec::new();
        for c in line.chars().take(self.number_of_parents) {
            let status = match c {
                '+' => DiffLineStatus::Added,
                '-' => DiffLineStatus::Removed,
                ' ' => DiffLineStatus::Unchanged,
                _ => return Ok(None),
            };

            from_status.push(status);
        }

        if from_status.len() != self.number_of_parents {
            return Ok(None);
        }
        let to_status = if from_status.contains(&DiffLineStatus::Added) {
            DiffLineStatus::Added
        } else if from_status
            .iter()
            .all(|status| *status == DiffLineStatus::Removed)
        {
            DiffLineStatus::Removed
        } else {
            DiffLineStatus::Unchanged
        };
        let content = Cow::Borrowed(line.get(self.number_of_parents..).unwrap_or(""));

        Ok(Some(DiffLine {
            from_status,
            to_status,
            content,
        }))
    }
}

#[test]
fn parse_single_parent_diff_lines_from_git_diff() {
    let parser = DiffLineParser {
        number_of_parents: 1,
    };
    assert_eq!(
        parser.parse_line_expected(" tmp*"),
        DiffLine {
            from_status: vec![DiffLineStatus::Unchanged],
            to_status: DiffLineStatus::Unchanged,
            content: Cow::Borrowed("tmp*"),
        },
    );
    assert_eq!(
        parser.parse_line_expected("-/.claude/"),
        DiffLine {
            from_status: vec![DiffLineStatus::Removed],
            to_status: DiffLineStatus::Removed,
            content: Cow::Borrowed("/.claude/"),
        },
    );
    assert_eq!(
        parser.parse_line_expected("+/.agents/"),
        DiffLine {
            from_status: vec![DiffLineStatus::Added],
            to_status: DiffLineStatus::Added,
            content: Cow::Borrowed("/.agents/"),
        },
    );
    assert_eq!(
        parser.parse_line_expected(
            "+    applicable.sort_by_key(|(line_number, _)| std::cmp::Reverse(*line_number));"
        ),
        DiffLine {
            from_status: vec![DiffLineStatus::Added],
            to_status: DiffLineStatus::Added,
            content: Cow::Borrowed(
                "    applicable.sort_by_key(|(line_number, _)| std::cmp::Reverse(*line_number));"
            ),
        },
    );
}

#[test]
fn parse_multi_parent_diff_lines() {
    let parser = DiffLineParser {
        number_of_parents: 2,
    };
    assert_eq!(
        parser.parse_line_expected("  unchanged"),
        DiffLine {
            from_status: vec![DiffLineStatus::Unchanged, DiffLineStatus::Unchanged],
            to_status: DiffLineStatus::Unchanged,
            content: Cow::Borrowed("unchanged"),
        },
    );
    assert_eq!(
        parser.parse_line_expected("- removed from first parent"),
        DiffLine {
            from_status: vec![DiffLineStatus::Removed, DiffLineStatus::Unchanged],
            to_status: DiffLineStatus::Unchanged,
            content: Cow::Borrowed("removed from first parent"),
        },
    );
    assert_eq!(
        parser.parse_line_expected("++ added against both parents"),
        DiffLine {
            from_status: vec![DiffLineStatus::Added, DiffLineStatus::Added],
            to_status: DiffLineStatus::Added,
            content: Cow::Borrowed(" added against both parents"),
        },
    );
}
