use anyhow::{Result, bail, ensure};

pub trait LineParser<'a>: Sized {
    type Output;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>>;

    fn parse_line_required(&self, line: &'a str) -> Result<Self::Output> {
        let Some(this) = self.parse_line(line)? else {
            bail!("could not parse required");
        };
        Ok(this)
    }

    #[cfg(test)]
    fn parse_line_expected(&self, line: &'a str) -> Self::Output {
        self.parse_line(line).unwrap().unwrap()
    }
}

pub trait Parser<'a>: Sized {
    type Output;

    fn parse(&self, s: &'a str) -> Result<Option<(Self::Output, &'a str)>>;

    fn parse_required(&self, s: &'a str) -> Result<(Self::Output, &'a str)> {
        let Some((this, rest)) = self.parse(s)? else {
            bail!("could not parse required");
        };
        Ok((this, rest))
    }

    #[cfg(test)]
    fn parse_expected(&self, line: &'a str) -> Self::Output {
        self.parse(line).unwrap().unwrap().0
    }
}

pub trait MultilineParser<'a>: Sized {
    type Output;

    fn parse_lines(&self, lines: &'a [&'a str]) -> Result<Option<(Self::Output, &'a [&'a str])>>;

    fn parse_lines_required(&self, lines: &'a [&'a str]) -> Result<(Self::Output, &'a [&'a str])> {
        let Some((this, rest)) = self.parse_lines(lines)? else {
            bail!("could not parse required");
        };
        Ok((this, rest))
    }

    #[cfg(test)]
    fn parse_lines_expected(&self, lines: &'a [&'a str]) -> Self::Output {
        self.parse_lines(lines).unwrap().unwrap().0
    }
}

impl<'a, P: Parser<'a>> LineParser<'a> for P {
    type Output = P::Output;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>> {
        let Some((this, rest)) = self.parse(line)? else {
            return Ok(None);
        };
        ensure!(rest.trim().is_empty(), "trailing content int line");
        Ok(Some(this))
    }
}
