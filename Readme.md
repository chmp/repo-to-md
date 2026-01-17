# `review-to-md`

Format GitHub pull request comments as markdown for LLM consumption.

## Overview

`review-to-md` fetches PR review comments from GitHub using the `gh` CLI and formats them as markdown code blocks with inline comments. This makes it easy to provide PR review context to LLMs for addressing feedback.

## Installation

### Prerequisites

- [GitHub CLI (`gh`)](https://cli.github.com/) must be installed and authenticated
- Rust toolchain for building from source

Build the binary from source:

```bash
cargo build --release
```

The binary will be available at `target/release/review-to-md` (or `review-to-md.exe` on Windows).

## Usage

### Basic usage

Auto-detect repository from git remote (requires git repository with a configured remote):

```bash
review-to-md <PR_NUMBER>
```

Example:
```bash
review-to-md 78
```

### Explicit repository

Specify owner and repository explicitly:

```bash
review-to-md <PR_NUMBER> --owner <OWNER> --repo <REPO>
```

Example:
```bash
review-to-md 78 --owner chmp --repo review-to-md
```

### Save to file

```bash
review-to-md 78 > pr-comments.md
```

## Output format

The tool generates markdown with a header, file sections, and code blocks with inline review comments:

````markdown
# Pull Request Review Comments

Please address the following review comments:

## `path/to/file.rs` - Lines 10-15

```rust
pub struct Config {
    pub field: String,
// <review user="reviewer">
// This should be renamed to...
// </review>
}
```
````

Each file section includes:
- A markdown heading with the file path in backticks and line range
- Code context from the diff hunk with syntax highlighting
- Review comments embedded as `<review user="...">...</review>` XML tags
- Language-specific comment prefixes (e.g., `//` for Rust, `#` for Python)

## How it works

1. Detects repository owner and name from `git remote get-url origin` (or uses provided arguments)
2. Fetches PR comments using `gh api /repos/{owner}/{repo}/pulls/{pr_id}/comments`
3. Parses the JSON response and groups comments by file
4. Extracts code context from diff hunks
5. Formats as markdown with language-appropriate syntax highlighting
