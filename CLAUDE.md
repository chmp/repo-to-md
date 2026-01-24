# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Project Overview

`repo-to-md` is a Rust CLI tool that fetches GitHub PR review comments via the
`gh` CLI and formats them as markdown code blocks with inline comments. The
output is designed for consumption by LLMs when addressing PR feedback.

## Development workflow

### Before finalizing work

**IMPORTANT**: Always run these commands before finalizing any work:

```bash
cargo fmt                   # Format code
cargo clippy --all-targets  # Lint for issues
cargo test                  # Run test suite
```

All three must pass with no warnings or errors. This is mandatory before
committing or submitting work.

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

Binary location: `target/release/repo-to-md` (or `repo-to-md.exe` on Windows)

### Run

Fetch comments with interactive review selection:

```bash
cargo run -- review --pr <PR_NUMBER>
cargo run -- review --pr <PR_NUMBER> --repo <OWNER/REPO>
```

Fetch comments from a specific review (skip interactive selection):

```bash
cargo run -- review --pr <PR_NUMBER> --review <REVIEW_ID>
cargo run -- review --pr <PR_NUMBER> --review 1
cargo run -- review --pr <PR_NUMBER> --review -1
```

Filter reviews by author:

```bash
cargo run -- review --pr <PR_NUMBER> --author <USERNAME>
cargo run -- review --pr <PR_NUMBER> --author <USERNAME> --review -1
cargo run -- review --pr <PR_NUMBER> --author @me
cargo run -- review --pr <PR_NUMBER> --author @me --review -1
```

Apply comments directly to source files as TODO comments:

```bash
cargo run -- review --pr <PR_NUMBER> --apply
cargo run -- review --pr <PR_NUMBER> --apply --force  # Skip uncommitted changes check
```

The `--apply` flag inserts review comments directly into the source files as TODO
comments using `<review>` tags. By default, it refuses to run if there are
uncommitted changes in the working directory. Use `--force` to bypass this check.

Fetch issues:

```bash
cargo run -- issue <ISSUE_NUMBER>
cargo run -- issue <ISSUE_NUMBER> --repo <OWNER/REPO>
```

### Install skill

**Note to Claude Code users**: The information below documents the skill
installation functionality. This is for user reference only - skill installation
should always be performed by the user, not automatically by Claude Code.

Install the Claude Code skill globally:

```bash
cargo run -- install
```

Install locally to current project (finds project root via .git or .claude
directory):

```bash
cargo run -- install --local
```

Skill installation locations:

- Global: `~/.claude/skills/review-to-md/`
- Local: `<project-root>/.claude/skills/review-to-md/` (where project root is
  found by walking up from current directory)

**Note to Claude Code**: The binary is called `repo-to-md`, but the skill
`review-to-md`. The binary is a general tool. The skill is designed to only
handle PR reviews.

### Temporary files and exploration scripts

Temporary files and exploration scripts should be placed in:

- `target/tmp/` - For build-related temporary files
- `tmp/` - For manual exploration scripts and outputs

Do not commit exploration scripts or their output files. Add them to
`.gitignore` if needed.

## Architecture

### Project structure

This is a Cargo workspace with a single member package `repo-to-md/`:

```
repo-to-md/src/
├── lib.rs              - Public API surface, re-exports
├── main.rs             - CLI entry point
├── cli/                - CLI command implementations
│   ├── mod.rs          - CLI struct, command dispatch
│   ├── review.rs       - Review command (fetch PR review comments)
│   ├── issue.rs        - Issue command (fetch GitHub issues)
│   ├── install_skill.rs - Skill installation command
│   └── query.rs        - Query command
├── client/             - GitHub API client
│   ├── mod.rs          - Module exports
│   ├── github.rs       - GitHub GraphQL API implementation
│   ├── mock.rs         - Mock client for testing
│   └── traits.rs       - Client traits and data types
├── repository.rs       - Git repository utilities and MockRepository
├── formatting.rs       - Markdown formatting logic
├── diff.rs             - Diff parsing utilities
├── language.rs         - Language detection and comment syntax
└── tests.rs            - Test suite

examples/               - Test fixtures with JSON inputs and expected markdown outputs
```

**Module responsibilities:**

- **main.rs** - Entry point, invokes CLI from lib
- **cli/** - Command implementations using argh for argument parsing
  - **review.rs** - Handles `--review`, `--author`, `--pr` options
  - **issue.rs** - Fetches and formats GitHub issues
- **client/** - GitHub API abstraction with trait-based design for testability
  - **github.rs** - Real implementation using `gh` CLI
  - **mock.rs** - MockGitHubClient for testing
  - **traits.rs** - Shared types (Review, Comment, Issue, PullRequest, etc.)
- **repository.rs** - Git operations (get remote URL, branch), MockRepository
- **formatting.rs** - Transforms comments/issues into markdown
- **diff.rs** - Parses unified diffs, extracts line numbers, handles truncation
- **language.rs** - Detects languages from file extensions, provides comment
  syntax
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

**GitHub GraphQL API (client/):**

- **Traits** (traits.rs):
  - `GetCurrentUserClient` - Get authenticated user
  - `ListReviewsClient` - List PR reviews
  - `FetchReviewCommentsClient` - Fetch comments from a review
  - `ListPullRequestsClient` - List open pull requests
  - `FetchIssueClient` - Fetch a single issue
- **Data types** (traits.rs): `Review`, `Comment`, `Issue`, `PullRequest`, `User`
- **GithubClient** (github.rs) - Real implementation using `gh api graphql`
- **MockGitHubClient** (mock.rs) - Test double with builder pattern:
  - `with_reviews()`, `with_comments()`, `with_issue()`, `with_pull_requests()`
- **Security**: GraphQL queries use parameterized variables (passed via `gh -F`
  flags), not string interpolation. The `gh` CLI handles escaping and injection
  prevention.

**GitHub GraphQL API Documentation:**

- [GraphQL Queries Reference](https://docs.github.com/en/graphql/reference/queries) -
  Available root-level queries including `viewer`
- [GraphQL Objects Reference](https://docs.github.com/en/graphql/reference/objects) -
  Object types like `User`, `PullRequest`, `Review`

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

**Test modules:**

- `tests::language_detection` - Language detection from file extensions
- `tests::comment_syntax` - Comment prefix/suffix for different languages
- `tests::diff_parsing` - Diff hunk parsing and code extraction
- `tests::issue_formatting` - Issue formatting tests with example files
- `tests::integration` - Review comment formatting tests with example files
- `tests::cli_end_to_end` - End-to-end CLI tests using MockGitHubClient and
  MockRepository. Tests the full command flow including:
  - Review command with various options (--pr, --review, --author, @me)
  - Issue command with repo auto-detection
  - PR auto-detection from current branch

**Testing approach:**

- Use `MockGitHubClient` and `MockRepository` for isolated CLI tests
- Commands provide `run_with_writer()` method to capture output for testing
- Test main output (stdout) but not user-directed logs (eprintln)

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

- Use sentence case for all headings (capitalize only the first word and proper
  nouns)
- Examples:
  - ✓ "Output format"
  - ✗ "Output Format"
  - ✓ "How it works"
  - ✗ "How It Works"
- Exception: Proper nouns and acronyms remain capitalized (e.g., "GitHub API
  usage")

## Claude Code skills

### Skills documentation

Skills are modular capabilities that extend Claude Code's functionality. For
complete documentation on creating and using skills:

- [Claude Code Skills Documentation](https://code.claude.com/docs/en/skills) -
  How to create, install, and use skills in Claude Code

### Skill installation locations

- **Global skills**: `~/.claude/skills/` - Available across all projects
- **Project skills**: `.claude/skills/` - Shared with team, version controlled

### repo-to-md skill

The repo-to-md skill is bundled with the binary and can be installed using:

```bash
repo-to-md install [--local]
```

The skill enables Claude Code to automatically use repo-to-md when working with
PR reviews.
