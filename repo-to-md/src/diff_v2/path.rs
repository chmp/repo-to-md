use std::borrow::Cow;

use anyhow::{Result, bail, ensure};

use super::parser::LineParser;

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct Path<'a>(pub Cow<'a, str>);

impl<'a> Path<'a> {
    pub(crate) fn borrowed(path: &'a str) -> Self {
        Self(Cow::Borrowed(path))
    }

    pub(crate) fn owned(path: impl Into<String>) -> Self {
        Self(Cow::Owned(path.into()))
    }

    pub fn into_static(self) -> Path<'static> {
        Path::owned(self.0.into_owned())
    }
}

// NOTE: due to the lossy conversion, only a line parser makes sense. Paths
// followed by content can only be disambiguated in context.
#[derive(Debug, Default, Clone, Copy)]
pub struct PathParser;

impl<'a> LineParser<'a> for PathParser {
    const NAME: &'static str = "path";

    type Output = Path<'a>;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>> {
        if let Some((path, rest)) = parse_quoted_path(line)? {
            if !rest.trim().is_empty() {
                bail!("trailing content in quoted path");
            }
            return Ok(Some(Path(Cow::Owned(path))));
        }
        let path = line.trim_end();
        ensure!(!path.is_empty(), "no path");
        Ok(Some(Path(Cow::Borrowed(path))))
    }
}

pub(super) fn parse_quoted_path(line: &str) -> Result<Option<(String, &str)>> {
    let Some(line) = line.strip_prefix('"') else {
        return Ok(None);
    };

    let mut bytes = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        match c {
            '"' => {
                let rest = &line[index + c.len_utf8()..];
                return Ok(Some((String::from_utf8(bytes)?, rest)));
            }
            '\\' => bytes.push(parse_quoted_path_escape(&mut chars)?),
            _ => {
                let mut char_bytes = [0; 4];
                bytes.extend_from_slice(c.encode_utf8(&mut char_bytes).as_bytes());
            }
        }
    }

    bail!("unterminated quoted path");
}

fn parse_quoted_path_escape(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Result<u8> {
    let Some((_, c)) = chars.next() else {
        bail!("unterminated quoted path escape");
    };

    let byte = match c {
        'a' => b'\x07',
        'b' => b'\x08',
        'f' => b'\x0c',
        'n' => b'\n',
        'r' => b'\r',
        't' => b'\t',
        'v' => b'\x0b',
        '\\' => b'\\',
        '"' => b'"',
        '0'..='7' => {
            let mut value = c as u16 - '0' as u16;
            for _ in 0..2 {
                let Some((_, next)) = chars.peek().copied() else {
                    break;
                };
                if !matches!(next, '0'..='7') {
                    break;
                }
                chars.next();
                value = value * 8 + (next as u16 - '0' as u16);
            }
            u8::try_from(value)?
        }
        _ => bail!("unsupported quoted path escape: \\{c}"),
    };
    Ok(byte)
}

#[test]
fn parse_path() {
    assert_eq!(
        PathParser.parse_line_expected("src/main.rs"),
        Path::borrowed("src/main.rs")
    );
    assert_eq!(
        PathParser.parse_line_expected("path with spaces.md  "),
        Path::borrowed("path with spaces.md")
    );
    assert_eq!(
        PathParser.parse_line_expected("\"path\\twith\\ncontrols.md\""),
        Path::owned("path\twith\ncontrols.md")
    );
    assert_eq!(
        PathParser.parse_line_expected("\"quote\\\"backslash\\\\.md\""),
        Path::owned("quote\"backslash\\.md")
    );
    assert_eq!(
        PathParser.parse_line_expected("\"utf8-\\302\\265.md\""),
        Path::owned("utf8-\u{00b5}.md")
    );
}
