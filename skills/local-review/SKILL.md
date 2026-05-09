---
name: local-review-comments
description: Retrieves local review comments from the review-comments.json file and formats them as markdown for LLM consumption. Use when the user asks to address local review comments or feedback.
---

# Retrieving local review comments

Use the `repo-to-md` CLI tool to format local review comments as LLM-friendly
markdown.

## Primary usage

```bash
repo-to-md review format --local
```

This reads `review-comments.json` from the current directory and outputs
markdown to stdout.

## Prerequisites

- A `review-comments.json` file must exist (created by `repo-to-md review local`)
- The file contains comments from a local review session

## Alternative usage

Specify a different comments file:

```bash
repo-to-md review format path/to/comments.json
```

Output to a file instead of stdout:

```bash
repo-to-md review format -o output.md
```

## Output format

Generates markdown with file sections containing code context and embedded
review comments:

````markdown
# Review comments

Please address the following review comments. The comments are given as inline
comments inside code blocks. They use the comment format of the language of the
commented file. Each comment is wrapped in "<review>" tags.

For example, a Rust file may look like:

```rust
fn main() {
    println!("13");
    // <review>
    // Please use 42 as the number
    // </review>
}
```

## General Comments

**@user:**

This is a general comment about the overall changes.

## `src/main.rs` - Line 10

```rust
pub fn main() {
// <review>
// Consider using clap for argument parsing
// </review>
    println!("Hello");
}
```
````

Each comment is embedded as `<review>...</review>` XML tags using the
language-specific comment syntax. Large diffs are intelligently truncated to
show only context around commented lines.
