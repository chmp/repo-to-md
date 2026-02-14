# `repo-to-md`

Format GitHub pull request reviews and issues as markdown for LLM consumption.

`repo-to-md` fetches PR review comments and issues from GitHub using the `gh`
CLI and formats them as markdown. Review comments are rendered as code blocks
with inline comments, while issues include their description and conversation
thread. This makes it easy to provide GitHub context to LLMs for addressing
feedback or implementing features.

## Installation

The create is currently not published. To install the tool, checkout the
repository and run

```bash
cargo install --path ./repo-to-md
```

## Usage

Prerequisites:

- [GitHub CLI (`gh`)](https://cli.github.com/) must be installed and
  authenticated
- Rust toolchain for building from source

To format the comments for pull request 78, use

```bash
repo-to-md review 78
```

This commands auto-detects the repository from git remote, if it configured. To
specify owner and repository explicitly, use:

```bash
repo-to-md review <PR_NUMBER> --owner <OWNER> --repo <REPO>
```

Example:

```bash
repo-to-md review 78 --owner chmp --repo repo-to-md
```

### AI Agent Skills

This tool includes skills that enable natural language interaction with GitHub
reviews and issues. The skills follow the [AgentSkills specification](https://agentskills.io/)
and can be used by any compatible AI agent.

**review-to-md** fetches and formats PR review comments. AI agents can use
this skill when addressing review feedback. Example prompts that
trigger this skill:

- "Please address my last review on GitHub"
- "Implement the feedback from my PR review"
- "Fix the issues mentioned in the code review"

**issue-to-md** fetches and formats GitHub issues. AI agents can use this
skill when working on issues. Example prompts that trigger this
skill:

- "Please implement issue 67"
- "What does GitHub issue #42 say?"
- "Help me implement the feature described in issue 123"

To install the skills, run:

```bash
# Install globally (available in all projects)
repo-to-md install

# Install locally (project-specific, finds project root via .git or .agents)
repo-to-md install --local

# Install to custom path
repo-to-md install --path /custom/skills/directory
```

Skills are installed to `~/.agents/skills/review-to-md/` (global) or
`<project-root>/.agents/skills/review-to-md/` (local).

### Review selection options

The default behavior auto-detects the PR from the current branch and selects
the last review. Use flags to override:

```bash
repo-to-md review --pr 78                      # Specific PR
repo-to-md review --review -1                   # Last review
repo-to-md review --author @me                  # Filter by author
repo-to-md review --repo owner/repo             # Override repository
```

For all available options, run `repo-to-md review --help`.

### Apply mode

Instead of outputting markdown, you can apply review comments directly to source
files:

```bash
repo-to-md review --apply              # Apply comments from current branch's PR
repo-to-md review --pr 78 --apply      # Apply comments from specific PR
```

This inserts comments directly into your source files using language-appropriate
comment syntax, wrapped in `<review>` tags:

```rust
fn main() {
    let x = 1;
    // <review>
    // Consider using a more descriptive name
    // </review>
}
```

By default, `--apply` refuses to run if you have uncommitted changes (to make it
easy to revert). Use `--force` to bypass this safety check.

```bash
repo-to-md review --apply --force      # Skip uncommitted changes check
```

### Fetching issues

Fetch and format a GitHub issue:

```bash
repo-to-md issue 42
repo-to-md issue 42 --repo owner/repo
```

The repository is auto-detected from the `origin` remote when not specified.

### Local review

Review local commits in a web UI before merging to the base branch:

```bash
repo-to-md local review                    # Auto-detect base, review commits up to HEAD
repo-to-md local review main               # Review commits from main to HEAD
repo-to-md local review main feature       # Review commits from main to feature
```

This launches a local web server with a side-by-side diff viewer where you can
add comments to the changes. The command reviews a range of commits (base..end)
and refuses to start if there are uncommitted changes when reviewing HEAD (use
`--force` to override). Comments are persisted to `review-comments.json` (use
`-o` to change). The diff and commit list are also persisted, so reopening the
session detects if commits have changed.

For all available options, run `repo-to-md local review --help`.

The comments from a local review session can be exported to markdown by running:

```bash
repo-to-md local format
```

To customize the review comments or the output to a file use

```bash
repo-to-md local format my-review.json -o review.md
```

## How it works

This tool uses the GitHub cli `gh` and the Git cli `git` for the following
operations

- `git`: read the configured remotes and the current branch
- `gh`: perform GraphQL queries using `gh api graphql`
