# `review-to-md`

Format GitHub pull request comments as markdown for LLM consumption.

## Overview

`review-to-md` fetches PR review comments from GitHub using the `gh` CLI and formats them as markdown code blocks with inline comments. This makes it easy to provide PR review context to LLMs for addressing feedback.

## Prerequisites

- [GitHub CLI (`gh`)](https://cli.github.com/) must be installed and authenticated
- Git repository with a configured remote (for auto-detection)

## Installation

Build the binary from source:

```bash
cargo build --release
```

The binary will be available at `target/release/review-to-md` (or `review-to-md.exe` on Windows).

## Usage

### Basic Usage

Auto-detect repository from git remote:

```bash
review-to-md <PR_NUMBER>
```

Example:
```bash
review-to-md 78
```

### Explicit Repository

Specify owner and repository explicitly:

```bash
review-to-md <PR_NUMBER> --owner <OWNER> --repo <REPO>
```

Example:
```bash
review-to-md 78 --owner chmp --repo markdown-app
```

### Save to File

```bash
review-to-md 78 > pr-comments.md
```

## Output Format

The tool generates markdown with code blocks showing the diff context and inline comments:

```markdown
## path/to/file.rs

```rust
pub struct Config {
    pub field: String,  // Comment (username): This should be renamed to...
}
```
```

Each comment includes:
- The file path as a heading
- Code context from the diff hunk
- Inline comments at the relevant lines with the commenter's username
- Language-specific comment syntax (e.g., `//` for Rust, `#` for Python)

## How It Works

1. Detects repository owner and name from `git remote get-url origin` (or uses provided arguments)
2. Fetches PR comments using `gh api /repos/{owner}/{repo}/pulls/{pr_id}/comments`
3. Parses the JSON response and groups comments by file
4. Extracts code context from diff hunks
5. Formats as markdown with language-appropriate syntax highlighting
