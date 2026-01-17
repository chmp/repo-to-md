# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Project Overview

`review-to-md` is a Rust CLI tool that fetches GitHub PR review comments via the
`gh` CLI and formats them as markdown code blocks with inline comments. The
output is designed for consumption by LLMs when addressing PR feedback.

## Commands

### Build

```bash
cargo build --release
```

Binary location: `target/release/review-to-md` (or `review-to-md.exe` on
Windows)

### Run

```bash
cargo run -- <PR_NUMBER>
cargo run -- <PR_NUMBER> --owner <OWNER> --repo <REPO>
cargo run -- --json-file examples/simple_comment.json
```

### Test

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

Run a specific test:

```bash
cargo test test_name
```

### Lint

```bash
cargo clippy
```

Always verify code quality with clippy before committing. The code should pass
with no warnings.

## Architecture

### Project Structure

This is a Cargo workspace with a single member package `review-to-md/`:

- `review-to-md/src/main.rs` - CLI entry point, argument parsing with `argh`
- `review-to-md/src/lib.rs` - Core library with all formatting logic
- `examples/` - Test fixtures with JSON inputs and expected markdown outputs

### Key Components

**Data Flow:**

1. Fetch PR comments from GitHub API via `gh` CLI or read from JSON file
2. Parse JSON into `Comment` structs (path, line number, body, diff_hunk, user)
3. Group comments by file path
4. For each file, process diff hunks and format as markdown code blocks

**Main Functions (lib.rs):**

- `fetch_pr_comments()` - Executes `gh api` command to fetch PR comments
- `parse_comments_json()` - Deserializes GitHub API JSON response
- `group_comments_by_file()` - Groups comments by their file path
- `format_comments_as_markdown()` - Core formatting engine that:
  - Groups comments by diff hunk within each file
  - Parses diff hunks to extract line numbers
  - Applies truncation for large diffs (shows CONTEXT_LINES=5 before/after
    commented lines)
  - Generates markdown with language-specific comment syntax
  - Embeds comments as `<review user="...">...</review>` XML tags

**Diff Hunk Parsing:**

- `parse_diff_hunk_with_line_numbers()` - Parses unified diff format (@@
  headers), tracks line numbers for added/context lines, supports optional line
  range filtering for truncation
- `calculate_context_range()` - Determines if a diff should be truncated based
  on MIN_TRUNCATION_THRESHOLD (20 lines) and calculates the range to display
- Line number tracking: added lines (+) and context lines increment the line
  counter, deleted lines (-) do not

**Language Detection:**

- `detect_language()` - Maps file extensions to markdown language identifiers
- `get_comment_prefix()` / `get_comment_suffix()` - Returns language-specific
  comment syntax (e.g., `//` for Rust, `#` for Python, `<!-- -->` for
  HTML/Markdown)

### Output Format

Comments are formatted as:

````
## `path/to/file.rs` - Lines 59-67

```rust
code line
// <review user="username">
// Comment text here
// Multi-line comments get prefix on each line
// </review>
````

Comments without line numbers appear at the top of the code block. Truncated
diffs show `...` at start/end.

### Testing Strategy

Tests use example JSON files in `examples/` with corresponding `.expected.md`
files. The `test_formatting()` helper compares actual output against expected
output.

## Documentation Style Guidelines

### Heading capitalization
- Use sentence case for all headings (capitalize only the first word and proper nouns)
- Examples:
  - ✓ "Output format"
  - ✗ "Output Format"
  - ✓ "How it works"
  - ✗ "How It Works"
- Exception: Proper nouns and acronyms remain capitalized (e.g., "GitHub API usage")
