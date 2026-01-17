# `review-to-md`

Format GitHub pull request comments as markdown for LLM consumption.

`review-to-md` fetches PR review comments from GitHub using the `gh` CLI and
formats them as markdown code blocks with inline comments. This makes it easy to
provide PR review context to LLMs for addressing feedback.

## Usage

Prerequisites:

- [GitHub CLI (`gh`)](https://cli.github.com/) must be installed and
  authenticated
- Rust toolchain for building from source

To format the comments for pull request 78, use

```bash
review-to-md 78
```

This commands auto-detects the repository from git remote, if it configured. To 
specify owner and repository explicitly, use:

```bash
review-to-md <PR_NUMBER> --owner <OWNER> --repo <REPO>
```

Example:

```bash
review-to-md 78 --owner chmp --repo review-to-md
```

### Review selection options

By default, the tool presents an interactive menu to select which review to process. Alternative selection methods:

**Direct review ID**:
```bash
review-to-md 78 --review-id PRR_kwDOAbcdef123456
```

**By index** (1-indexed, -1 for last review):
```bash
review-to-md 78 --review-index 1     # First review
review-to-md 78 --review-index -1    # Last review
```

**Filter by author**:
```bash
review-to-md 78 --author username    # Select from reviews by 'username'
```

**Combine filters**:
```bash
review-to-md 78 --author username --review-index -1  # Last review by 'username'
```

If `--author` filters to exactly one review, it will be auto-selected (no interactive prompt).

**From JSON file** (for testing/offline use):
```bash
review-to-md --json-file examples/simple_comment.json
```

## Output format

The tool generates markdown with a header, file sections, and code blocks with
inline review comments:

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
2. Fetches available reviews for the PR using GitHub GraphQL API via `gh api graphql`
3. Selects a specific review either:
   - Interactively via numbered menu (default)
   - By review ID using `--review-id`
   - By index using `--review-index` (optionally filtered by `--author`)
4. Fetches all comments from the selected review via GraphQL
5. Groups comments by file and diff hunk
6. Formats as markdown with language-appropriate syntax highlighting and embedded review comments
