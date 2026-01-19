---
name: fetching-pr-reviews
description: Fetches GitHub pull request review comments for the current repository and formats them as markdown for LLM consumption. Use when the user asks to fetch or address PR review comments or feedback.
---

# Fetching PR reviews

Fetches PR review comments for the git project from GitHub and formats them as
LLM-friendly markdown.

## Primary usage

Get the last review in GitHub by the current user for a PR in the git repository

```bash
repo-to-md review <PR_NUMBER> --author @me --review-index -1
```

The command can be executed anywhere in the repository. There is no need to
change the working directory.

## Prerequisites

If the command fails, please note potential failure cases:

- `gh` CLI must be installed and authenticated
- `repo-to-md` must be run from within a git repository with a configured
  remote

The tool auto-detects the repository from `git remote get-url origin` and the
current user from the authentication used for the `gh` command.

## Alternative usage

Get the last review by a specific user:

```bash
repo-to-md review <PR_NUMBER> --author <USERNAME> --review-index -1
```

Presents a numbered menu to select from available reviews.

**Filter by author** (interactive if multiple reviews):

```bash
repo-to-md review <PR_NUMBER> --author <USERNAME>
repo-to-md review <PR_NUMBER> --author @me  # The current user
```

**By index** (1-indexed, -1 for last):

```bash
repo-to-md review <PR_NUMBER> --review-index -1
```

## Non-recommended usage

`repo-to-md` supports selecting the review interactively, by omitting the
corresponding arguments.

```bash
repo-to-md review <PR_NUMBER>
```

**Please avoid this usage.**

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
