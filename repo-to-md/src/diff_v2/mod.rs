//! Parse diff output from git

mod chunk;
mod chunk_header;
mod diff;
mod diff_file;
mod diff_file_header;
mod diff_header;
mod diff_line;
mod extended_header_line;
mod file_header_line;
mod hash;
mod header_block;
mod index_line;
mod mode;
mod parser;
mod path;
mod percentage;

pub use chunk::{Chunk, ChunkParser};
pub use chunk_header::{ChunkHeader, ChunkHeaderParser};
pub use diff::{Diff, DiffParser, parse};
pub use diff_file::{DiffFile, DiffFileParser};
pub use diff_file_header::{DiffFileHeader, DiffFileHeaderParser};
pub use diff_header::{DiffHeader, DiffHeaderParser};
pub use diff_line::{DiffLine, DiffLineParser, DiffLineStatus};
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
