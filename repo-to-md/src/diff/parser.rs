//! Parser traits
//!
//! Parsers are separate values from the data they produce so parser instances can
//! carry configuration, such as the number of parents in a combined diff.
//!
//! The parser contract has three outcomes:
//!
//! - `Ok(Some((output, rest)))`: the input matched matched and was parsed
//!   successfully. The unconsumed input is included as the second part of the
//!   returned tuple
//! - `Ok(None)`: the input did not match this parser at all. Callers may try a
//!   different parser without treating this as an error.
//! - `Err(error)`: the input appeared to match this parser but was malformed.
//!
//! There are three levels of parsers:
//!
//! - [`Parser`]: parses content within a line
//! - [`LineParser`]: parses a full line
//! - [`MultiLineParser`]: parses sequences of lines
//!
//! There are blanket impls for the next higher level:
//!
//! - The [`LineParser`] blanket impl for [`Parser`] assumes the target makes
//!   up the full line. Only trailing whitespace is allowed
//! - The [`MultiLineParser`] blanket impl for [`LineParser`] tries to parse
//!   the first line

use anyhow::{Result, bail, ensure};

/// Parses a prefix of a string.
///
/// The returned `&str` is the unconsumed suffix.
pub trait Parser<'a>: Sized {
    const NAME: &'static str;

    type Output;

    fn parse(&self, s: &'a str) -> Result<Option<(Self::Output, &'a str)>>;

    fn parse_required(&self, s: &'a str) -> Result<(Self::Output, &'a str)> {
        let Some((this, rest)) = self.parse(s)? else {
            bail!("could not parse required {}", Self::NAME);
        };
        Ok((this, rest))
    }

    #[cfg(test)]
    fn parse_expected(&self, line: &'a str) -> Self::Output {
        self.parse(line).unwrap().unwrap().0
    }
}

/// Parses a complete line.
pub trait LineParser<'a>: Sized {
    const NAME: &'static str;

    type Output;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>>;

    fn parse_line_required(&self, line: &'a str) -> Result<Self::Output> {
        let Some(this) = self.parse_line(line)? else {
            bail!("could not parse required {}", Self::NAME);
        };
        Ok(this)
    }

    #[cfg(test)]
    fn parse_line_expected(&self, line: &'a str) -> Self::Output {
        self.parse_line(line).unwrap().unwrap()
    }
}

impl<'a, P: Parser<'a>> LineParser<'a> for P {
    const NAME: &'static str = P::NAME;

    type Output = P::Output;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>> {
        let Some((this, rest)) = self.parse(line)? else {
            return Ok(None);
        };
        ensure!(rest.trim().is_empty(), "trailing content int line");
        Ok(Some(this))
    }
}

/// Parses a sequence of complete lines.
///
/// The returned slice contains the unconsumed lines
pub trait MultilineParser<'a>: Sized {
    const NAME: &'static str;

    type Output;

    fn parse_lines(&self, lines: &'a [&'a str]) -> Result<Option<(Self::Output, &'a [&'a str])>>;

    fn parse_lines_required(&self, lines: &'a [&'a str]) -> Result<(Self::Output, &'a [&'a str])> {
        let Some((this, rest)) = self.parse_lines(lines)? else {
            bail!("could not parse required {}", Self::NAME);
        };
        Ok((this, rest))
    }

    fn parse_lines_many(&self, lines: &'a [&'a str]) -> Result<(Vec<Self::Output>, &'a [&'a str])> {
        let mut result = Vec::new();
        let mut rest = lines;

        loop {
            let Some((item, next_rest)) = self.parse_lines(rest)? else {
                break;
            };
            result.push(item);
            rest = next_rest;
        }

        Ok((result, rest))
    }

    #[cfg(test)]
    fn parse_lines_expected(&self, lines: &'a [&'a str]) -> Self::Output {
        self.parse_lines(lines).unwrap().unwrap().0
    }
}

impl<'a, P: LineParser<'a>> MultilineParser<'a> for P {
    const NAME: &'static str = P::NAME;

    type Output = P::Output;

    fn parse_lines(&self, lines: &'a [&'a str]) -> Result<Option<(Self::Output, &'a [&'a str])>> {
        let Some((head, tail)) = lines.split_first() else {
            return Ok(None);
        };
        let Some(item) = self.parse_line(head)? else {
            return Ok(None);
        };
        Ok(Some((item, tail)))
    }
}
