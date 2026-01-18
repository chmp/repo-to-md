---
name: fetching-pr-reviews
description: Fetches GitHub pull request review comments for the current repository and formats them as markdown for LLM consumption. Use when the user asks to fetch or address PR review comments or feedback.
---

# Fetching PR reviews

Fetches PR review comments for the current repository and formats them as
LLM-friendly markdown.

## Primary usage

**Get the last review by a specific user**:

```bash
review-to-md <PR_NUMBER> --author <USERNAME> --review-index -1
```

This is the most common workflow: fetch the most recent review from a specific
reviewer for a PR in the current repository.

## Prerequisites

- `gh` CLI must be installed and authenticated
- Must be run from within a git repository with a configured remote

The tool auto-detects the repository from `git remote get-url origin`.

## Alternative usage

**Interactive selection**:

```bash
review-to-md <PR_NUMBER>
```

Presents a numbered menu to select from available reviews.

**Filter by author** (interactive if multiple reviews):

```bash
review-to-md <PR_NUMBER> --author <USERNAME>
```

**By index** (1-indexed, -1 for last):

```bash
review-to-md <PR_NUMBER> --review-index -1
```

## Output format

Generates markdown with file sections containing code context and embedded
review comments:

````markdown
## `src/main.rs` - Lines 10-15

```rust
pub fn main() {
// <review user="reviewer">
// Consider using clap for argument parsing
// </review>
    println!("Hello");
}
```
````

Each comment is embedded as `<review user="...">...</review>` XML tags using the
language-specific comment syntax.
