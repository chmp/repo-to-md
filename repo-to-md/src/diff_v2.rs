//! Parse diff output from git

mod diff_header;
mod extended_header_line;
mod file_header_line;
mod hash;
mod header_block;
mod index_line;
mod mode;
mod parser;
mod path;
mod percentage;

pub use diff_header::{DiffHeader, DiffHeaderParser};
pub use extended_header_line::{ExtendedHeaderLine, ExtendedHeaderLineParser};
pub use file_header_line::{
    FileHeaderLine, FileHeaderLineParser, NewFileHeaderLine, NewFileHeaderLineParser,
    OldFileHeaderLine, OldFileHeaderLineParser,
};
pub use hash::{Hash, HashParser};
pub use header_block::{DiffHeaderBlock, DiffHeaderBlockParser};
pub use index_line::{IndexLine, IndexLineParser};
pub use mode::{Mode, ModeParser};
pub use parser::{LineParser, MultilineParser, Parser};
pub use path::{Path, PathParser};
pub use percentage::{Percentage, PercentageParser};

pub fn parse_diff_output(_lines: &[String]) {}
