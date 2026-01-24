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

### Claude Code skills

This tool includes two Claude Code skills that enable natural language
interaction with GitHub reviews and issues.

**review-to-md** fetches and formats PR review comments. Claude Code will use
this skill when you ask it to address review feedback. Example prompts that
trigger this skill:

- "Please address my last review on GitHub"
- "Implement the feedback from my PR review"
- "Fix the issues mentioned in the code review"

**issue-to-md** fetches and formats GitHub issues. Claude Code will use this
skill when you ask it to work on an issue. Example prompts that trigger this
skill:

- "Please implement issue 67"
- "What does GitHub issue #42 say?"
- "Help me implement the feature described in issue 123"

To install the skills, run:

```bash
# Install globally (available in all projects)
repo-to-md install

# Install locally (project-specific, finds project root via .git or .claude)
repo-to-md install --local
```

Skills are installed to `~/.claude/skills/review-to-md/` (global) or
`<project-root>/.claude/skills/review-to-md/` (local).

### Review selection options

Per default the PR matching the current branch is selected, to explicitly select
a review, use

```bash
repo-to-md review --pr 78
```

Per default the last review on a PR is selected, to explicitly select a review
either by ID or by index, use

```bash
repo-to-md review --review PRR_kwDOAbcdef123456 # Fixed id
repo-to-md review --review 1                    # First review
repo-to-md review --review -1                   # Last review
```

Per default the reviews considered when selecting by index are filterted by the
current user, to select reviews by another user use

```bash
repo-to-md review --author username    # Select from reviews by 'username'
repo-to-md review --author @me         # Select from your own reviews
```

Per default the GitHub repository for the `origin` remote is used, to overwrite
the repo, use

```bash
repo-to-md review --repo owner/repo
```

Note that, when specifying a review id directly, other review filters are
ignored. All filters can be combined, for example

```bash
repo-to-md review --pr 78 --author username --review-index -1  # Last review by 'username' on pr 78
```

### Apply mode

Instead of outputting markdown, you can apply review comments directly to source
files:

```bash
repo-to-md review --apply              # Apply comments from current branch's PR
repo-to-md review --pr 78 --apply      # Apply comments from specific PR
repo-to-md review --apply --force      # Skip uncommitted changes check
```

This inserts comments directly into your source files using language-appropriate
comment syntax, wrapped in `<review user="...">` tags:

```rust
fn main() {
    let x = 1;
    // <review user="reviewer">
    // Consider using a more descriptive name
    // </review>
}
```

By default, `--apply` refuses to run if you have uncommitted changes (to make it
easy to revert). Use `--force` to bypass this safety check.

### Fetching issues

Fetch and format a GitHub issue:

```bash
repo-to-md issue 42
repo-to-md issue 42 --repo owner/repo
```

The repository is auto-detected from the `origin` remote when not specified.

## How it works

This tool uses the GitHub cli `gh` and the Git cli `git` for the following
operations

- `git`: read the configured remotes and the current branch
- `gh`: perform GraphQL queries using `gh api graphql`
