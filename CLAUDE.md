# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Project Overview

`review-to-md` is a Rust CLI tool that fetches GitHub PR review comments via the
`gh` CLI and formats them as markdown code blocks with inline comments. The
output is designed for consumption by LLMs when addressing PR feedback.

## Development workflow

### Before finalizing work

**IMPORTANT**: Always run these commands before finalizing any work:

```bash
cargo fmt      # Format code
cargo clippy   # Lint for issues
cargo test     # Run test suite
```

All three must pass with no warnings or errors. This is mandatory before committing or submitting work.

### Format

```bash
cargo fmt
```

Format all code in the workspace:

```bash
cargo fmt --all
```

Check formatting without modifying files (useful for CI):

```bash
cargo fmt --all -- --check
```

Always format code before committing. The CI workflow will fail if code is not
properly formatted.

### Lint

```bash
cargo clippy
```

Always verify code quality with clippy before committing. The code should pass
with no warnings.

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

### Build

```bash
cargo build --release
```

Binary location: `target/release/review-to-md` (or `review-to-md.exe` on
Windows)

### Run

Fetch comments with interactive review selection:

```bash
cargo run -- <PR_NUMBER>
cargo run -- <PR_NUMBER> --owner <OWNER> --repo <REPO>
```

Fetch comments from a specific review (skip interactive selection):

```bash
cargo run -- <PR_NUMBER> --review-id <REVIEW_ID>
cargo run -- <PR_NUMBER> --review-index 1
cargo run -- <PR_NUMBER> --review-index -1
```

Filter reviews by author:

```bash
cargo run -- <PR_NUMBER> --author <USERNAME>
cargo run -- <PR_NUMBER> --author <USERNAME> --review-index -1
```

Read from JSON file:

```bash
cargo run -- --json-file examples/simple_comment.json
```

### Temporary files and exploration scripts

Temporary files and exploration scripts should be placed in:
- `target/tmp/` - For build-related temporary files
- `tmp/` - For manual exploration scripts and outputs

Do not commit exploration scripts or their output files. Add them to `.gitignore` if needed.

## Architecture

### Project structure

This is a Cargo workspace with a single member package `review-to-md/`:

```
review-to-md/src/
├── lib.rs           - Public API surface, re-exports
├── main.rs          - CLI entry point, argument parsing, interactive review selection
├── client.rs        - GitHub GraphQL API client
├── formatting.rs    - Markdown formatting logic
├── diff.rs          - Diff parsing utilities
├── language.rs      - Language detection and comment syntax
└── tests.rs         - Test suite

examples/            - Test fixtures with JSON inputs and expected markdown outputs
```

**Module responsibilities:**

- **main.rs** - CLI interface, handles `--review-id` option and interactive review
  selection
- **client.rs** - GraphQL queries via `gh` CLI, fetches reviews and comments
- **formatting.rs** - Transforms comments into markdown with inline code blocks
- **diff.rs** - Parses unified diffs, extracts line numbers, handles truncation
- **language.rs** - Detects languages from file extensions, provides comment syntax
- **lib.rs** - Public API, re-exports key functions and types

### Key components

**Data flow:**

1. **Interactive mode** (default):
   - Fetch all reviews for PR via GraphQL (`list_reviews`)
   - Display review table with author, date, comment count, and description
   - User selects review by number
   - Fetch comments from selected review via GraphQL (`fetch_review_comments`)
2. **Direct mode** (`--review-id` provided):
   - Fetch comments from specified review via GraphQL
3. **File mode** (`--json-file` provided):
   - Read comments from local JSON file
4. Group comments by file path
5. Format as markdown code blocks with inline comments

**GitHub GraphQL API (client.rs):**

- `list_reviews()` - Fetches all reviews for a PR with metadata (ID, author,
  state, body, created date, comment count)
- `fetch_review_comments()` - Fetches all comments from a specific review by ID
- GraphQL queries via `gh api graphql` command
- Returns `Review` and `Comment` structs
- Includes `isMinimized` field (fetched but not used yet)
- **Security**: GraphQL queries use parameterized variables (passed via `gh -F` flags),
  not string interpolation. The `gh` CLI handles escaping and injection prevention.
  Variables are never interpolated directly into the query string.

**Formatting engine (formatting.rs):**

- `format_comments_as_markdown()` - Core formatting logic:
  - Groups comments by diff hunk within each file
  - Parses diff hunks to extract line numbers
  - Applies truncation for large diffs (shows CONTEXT_LINES=5 before/after
    commented lines)
  - Generates markdown with language-specific comment syntax
  - Embeds comments as `<review user="...">...</review>` XML tags

**Diff parsing (diff.rs):**

- `parse_diff_hunk_with_line_numbers()` - Parses unified diff format (@@
  headers), tracks line numbers for added/context lines, supports optional line
  range filtering for truncation
- `calculate_context_range()` - Determines if a diff should be truncated based
  on MIN_TRUNCATION_THRESHOLD (20 lines) and calculates the range to display
- Line number tracking: added lines (+) and context lines increment the line
  counter, deleted lines (-) do not

**Language detection (language.rs):**

- `detect_language()` - Maps file extensions to markdown language identifiers
- `get_comment_prefix()` / `get_comment_suffix()` - Returns language-specific
  comment syntax (e.g., `//` for Rust, `#` for Python, `<!-- -->` for
  HTML/Markdown)

### Output format

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

### Testing strategy

Tests use example JSON files in `examples/` with corresponding `.expected.md`
files. The `test_formatting()` helper compares actual output against expected
output.

**Running tests:**

```bash
cargo test
```

**Module-specific tests:**

- `tests::language_detection` - Language detection from file extensions
- `tests::comment_syntax` - Comment prefix/suffix for different languages
- `tests::diff_parsing` - Diff hunk parsing and code extraction
- `tests::integration` - End-to-end formatting tests with example files

## Code organization guidelines

### Module structure

Organize code in this order:

1. **Public types** - Exported structs and enums
2. **Internal types** - Private implementation details
3. **Public functions** (each followed by their specific support functions)
4. **General support functions** - Helpers used by multiple public functions

**Example from client.rs**:
```rust
// 1. Public types
pub struct Review { ... }
pub struct ReviewAuthor { ... }

// 2. Internal types
struct GraphQLResponse<T> { ... }
struct ListReviewsData { ... }

// 3. Public functions
pub fn list_reviews(...) { ... }
pub fn fetch_review_comments(...) { ... }

// 4. General support functions
fn run_graphql_query(...) { ... }
```

### Function ordering in main.rs

1. Imports
2. Argument structs (Cli with argh derives)
3. Main function
4. Support functions (in logical order)

### Public interface changes

When modifying the public CLI interface (adding/removing/changing flags):

1. Update Readme.md with user-facing documentation
2. Update CLAUDE.md with developer-facing details
3. Update help text in argh derive attributes
4. Add tests if the change affects behavior

## Documentation Style Guidelines

### Heading capitalization
- Use sentence case for all headings (capitalize only the first word and proper nouns)
- Examples:
  - ✓ "Output format"
  - ✗ "Output Format"
  - ✓ "How it works"
  - ✗ "How It Works"
- Exception: Proper nouns and acronyms remain capitalized (e.g., "GitHub API usage")
