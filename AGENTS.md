# AGENTS.md

This file provides guidance for AI agents when working with code in this
repository. The tool follows the [AgentSkills specification](https://agentskills.io/)
for skill compatibility.

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

Run frontend browser tests:

```bash
nix run .#frontend-test
```

The frontend test runner serves `repo-to-md/src/static/test.html` locally and
executes it in headless Chromium via Python Playwright. The flake provides the
Nix-packaged Playwright browser bundle required on NixOS.

### Build

```bash
cargo build --release
```

Binary location: `target/release/repo-to-md` (or `repo-to-md.exe` on Windows)

### Run

This section describes the functionality of CLI. Do not run these commands,
unless explicitly prompted.

Fetch comments with interactive review selection:

```bash
cargo run -- review format
cargo run -- review format --repo <OWNER/REPO>
```

The `format` subcommand may be omitted when the first argument is not a known
review subcommand, so `cargo run -- review <PR_NUMBER>` is accepted as shorthand
for `cargo run -- review format <PR_NUMBER>`.

Fetch comments from a specific review (skip interactive selection):

```bash
cargo run -- review format <PR_NUMBER> --review <REVIEW_ID>
cargo run -- review format <PR_NUMBER> --review 1
cargo run -- review format <PR_NUMBER> --review -1
```

Filter reviews by author:

```bash
cargo run -- review format --author <USERNAME>
cargo run -- review format <PR_NUMBER> --author <USERNAME> --review -1
cargo run -- review format --author @me
cargo run -- review format <PR_NUMBER> --author @me --review -1
```

Fetch issues:

```bash
cargo run -- issue format <ISSUE_NUMBER>
cargo run -- issue format <ISSUE_NUMBER> --repo <OWNER/REPO>
```

The `format` subcommand may be omitted when the first argument is not a known
issue subcommand, so `cargo run -- issue <ISSUE_NUMBER>` is accepted as shorthand
for `cargo run -- issue format <ISSUE_NUMBER>`.

Review local changes with web UI:

```bash
cargo run -- review local                    # Auto-detect base, review commits up to HEAD
cargo run -- review local main               # Review commits from main to HEAD
cargo run -- review local main feature       # Review commits from main to feature
cargo run -- review local HEAD~5 HEAD~2      # Review specific commit range
cargo run -- review local main --no-open     # Don't open browser automatically
cargo run -- review local --force            # Force regeneration even with uncommitted changes
```

The `review local` command launches a local web server with a side-by-side diff
viewer for reviewing a range of commits before merge. It takes a base ref (first
argument) and an optional end ref (second argument, defaults to HEAD). When no
arguments are provided, it auto-detects the base branch (trying origin/HEAD,
main, then master).

The command refuses to start if:
- There are uncommitted changes and reviewing HEAD (use `--force` to override)
- The session file exists but has changed commits/refs (use `--force` to regenerate)
- No commits exist in the range

Comments are saved to `review-comments.json` by default (use `-o` to change).
Browser opens automatically by default (use `--no-open` to disable). The session
file tracks commits so reopening detects if the branch has changed.

The bind address and port default to `127.0.0.1` and `8080`. They can be set
with `REPO_TO_MD_BIND` and `REPO_TO_MD_PORT`; explicit `--bind` and `--port`
arguments take precedence over the environment.

The server can be stopped by:
- Pressing Ctrl-C in the terminal
- Clicking the "Stop Server" button in the web UI

On shutdown, the server prints the `review format` command to run next.

Format comments as markdown:

```bash
cargo run -- review format review-comments.json     # Format specific file to stdout
cargo run -- review format -o out.md                # Format to file
```

When the positional argument names an existing path, `review format` treats it as
a local comments file. Otherwise it is treated as a GitHub review ID or review
index.

### Install skill

Install the skills globally:

```bash
cargo run -- install
```

Install locally to current project (finds project root via .git or .agents
directory):

```bash
cargo run -- install --local
```

Install to a custom path:

```bash
cargo run -- install --path /custom/skills/directory
```

Skill installation locations:

- Global: `~/.agents/skills/review-to-md/`
- Local: `<project-root>/.agents/skills/review-to-md/` (where project root is
  found by walking up from current directory)
- Custom: Any path specified with `--path` option

The skills follow the [AgentSkills specification](https://agentskills.io/) and
can be used by any compatible AI agent.

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
│   ├── review.rs       - Review supercommand
│   ├── review_format.rs - Format remote and local review comments
│   ├── review_local.rs - Local review web UI command
│   ├── issue.rs        - Issue supercommand
│   ├── issue_format.rs - Format GitHub issues
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
├── tests.rs            - Test suite
└── static/             - Frontend assets for local review UI
    ├── index.html      - Main UI page
    ├── test.html       - Frontend test runner
    ├── styles.css      - Stylesheet
    └── js/
        ├── api.js          - API client for backend communication
        ├── app.js          - Main application coordinator
        ├── utils.js        - Shared pure utility functions
        ├── diff-view.js    - <diff-view> custom element
        ├── file-tree.js    - <file-tree> custom element
        ├── comment-form.js - <comment-form> custom element
        ├── review-comment.js - <review-comment> custom element
        └── test/
            ├── minitest.js      - Test framework
            ├── utils.test.js    - Pure function tests
            └── components.test.js - DOM component tests

examples/               - Test fixtures with JSON inputs and expected markdown outputs
```

**Rust module responsibilities:**

- **main.rs** - Entry point, invokes CLI from lib
- **cli/** - Command implementations using argh for argument parsing
  - **review.rs** - Review supercommand
  - **review_format.rs** - Formats GitHub and local review comments
  - **review_local.rs** - Launches the local review web UI
  - **issue.rs** - Issue supercommand
  - **issue_format.rs** - Fetches and formats GitHub issues
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

**Frontend module responsibilities** (in `static/js/`):

- **api.js** - HTTP client for backend API (fetch session, CRUD comments)
- **app.js** - Main coordinator, event handling, state management
- **utils.js** - Pure utility functions extracted for testability:
  - `escapeHtml`, `escapeAttr` - XSS prevention
  - `getRowType`, `groupCommentsByLine`, `getCommentsByFile` - Data transforms
  - `getFileName`, `formatDate` - String/date utilities
- **diff-view.js** - Side-by-side diff display with inline comments
- **file-tree.js** - File list with status icons and comment counts
- **comment-form.js** - New comment input form
- **review-comment.js** - Comment display with edit/delete actions

### Key components

**Data flow:**

1. **Interactive mode** (default):
   - Fetch all reviews for PR via GraphQL (`list_reviews`)
   - Display review table with author, date, comment count, and description
   - User selects review by number
   - Fetch comments from selected review via GraphQL (`fetch_review_comments`)
2. **Direct mode** (review ID positional provided):
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
- **Data types** (traits.rs): `Review`, `Comment`, `Issue`, `PullRequest`,
  `User`
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
  - Embeds comments as `<review>...</review>` XML tags

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
// <review>
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

**Running Rust tests:**

```bash
cargo test
```

**Running frontend tests:**

Frontend tests run in the browser using the minitest.js framework. To run them:

1. Start the local server: `cargo run -- review local HEAD~1`
2. Navigate to `http://localhost:PORT/test.html` (replace PORT with actual port)
3. Open browser developer tools (F12) and check the Console tab
4. Green = passed, Red = failed

Alternatively, open `repo-to-md/src/static/test.html` directly in a browser (some
tests may fail due to module loading restrictions without a server).

**Rust test modules:**

- `tests::language_detection` - Language detection from file extensions
- `tests::comment_syntax` - Comment prefix/suffix for different languages
- `tests::diff_parsing` - Diff hunk parsing and code extraction
- `tests::issue_formatting` - Issue formatting tests with example files
- `tests::integration` - Review comment formatting tests with example files
- `tests::cli_end_to_end` - End-to-end CLI tests using MockGitHubClient and
  MockRepository. Tests the full command flow including:
  - Review command with various options (review ID/index, --author, @me)
  - Issue command with repo auto-detection
  - PR auto-detection from current branch

**Frontend test modules** (in `repo-to-md/src/static/js/test/`):

- `utils.test.js` - Tests for pure utility functions:
  - `escapeHtml`, `escapeAttr` - XSS prevention (security critical)
  - `getRowType` - Diff row classification
  - `groupCommentsByLine`, `getCommentsByFile` - Comment grouping
  - `getFileName`, `formatDate` - String/date utilities
- `components.test.js` - DOM-based tests for custom elements:
  - `CommentForm` - Submit, cancel, keyboard shortcuts
  - `ReviewComment` - Edit, save, delete, XSS escaping

**Testing approach:**

- **Prefer snapshot tests** using `insta` for complex output verification
- Use `insta::assert_snapshot!()` instead of multiple `assert!(contains())`
- Run `cargo insta review` to approve new snapshots
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
2. Update AGENTS.md with developer-facing details
3. Update help text in argh derive attributes
4. Add tests if the change affects behavior

### Keeping AGENTS.md current

Always update AGENTS.md when making changes that affect:

- **CLI interface** - New commands, changed flags, renamed subcommands
- **Project structure** - New modules, reorganized directories, moved files
- **Build/test commands** - Changed cargo commands, new test patterns
- **Key architectural decisions** - New patterns, changed data flow

AGENTS.md serves as the primary reference for AI assistants working with this
codebase. Outdated documentation leads to incorrect assumptions and wasted
effort.

### Error handling and unwrap

Never use `.unwrap()` in non-test code. Instead:

1. **For fallible operations**: Propagate errors with `?` or handle with
   `match`/`if let`

   ```rust
   // Preferred: propagate error
   let file = File::open(path)?;

   // Preferred: handle error explicitly
   let file = match File::open(path) {
       Ok(f) => f,
       Err(e) => return Err(e.into()),
   };
   ```

2. **For infallible operations** (e.g., accessing non-empty collections after a
   check): Use `let ... else { unreachable!() }`

   ```rust
   if !items.is_empty() {
       let Some(first) = items.first() else {
           unreachable!("items is non-empty");
       };
       // use first
   }
   ```

3. **For lock poisoning** (RwLock/Mutex): Use `.expect("lock poisoned")` since
   poisoned locks indicate a panic elsewhere and cannot be meaningfully
   recovered

   ```rust
   let guard = self.data.read().expect("lock poisoned");
   ```

4. **In tests**: `.unwrap()` is acceptable since test failures are expected to
   panic

### Format macro style

Use named arguments in format-style macros for clarity. This applies to:
`format!`, `println!`, `eprintln!`, `print!`, `eprint!`, `write!`, `writeln!`,
`bail!`, `anyhow!`, and similar macros.

```rust
// Preferred: named arguments
eprintln!("Processing {file} on line {line}", file = path, line = num);
format!("Error: {message}", message = err);
bail!("Failed to open {path}: {error}", path = file_path, error = e);

// Avoid: positional arguments
eprintln!("Processing {} on line {}", path, num);
format!("Error: {}", err);
bail!("Failed to open {}: {}", file_path, e);
```

For simple single-variable cases, inline capture is acceptable:

```rust
let name = "test";
println!("Hello {name}");  // OK - single variable, inline capture
```

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

## AI Agent Skills

### Skills documentation

Skills are modular capabilities that follow the [AgentSkills specification](https://agentskills.io/)
for compatibility with various AI agents.

### Skill installation locations

- **Global skills**: `~/.agents/skills/` - Available across all projects
- **Project skills**: `.agents/skills/` - Shared with team, version controlled
- **Custom skills**: Any path specified with `--path` option

**Important**: The `skills/` directory at the repo root contains the source
skill files. The `.agents/skills/` directory contains installed copies generated
by `repo-to-md install --local`. Always edit skills in `skills/`, never in
`.agents/skills/` directly.

### repo-to-md skill

The repo-to-md skill is bundled with the binary and can be installed using:

```bash
repo-to-md install [--local] [--path /custom/path]
```

The skill enables AI agents to automatically use repo-to-md when working with
PR reviews.
