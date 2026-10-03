# `repo-to-md` - Markdown based Git workflows

`repo-to-md` allows agents to interact with reviews and issues. It supports
GitHub pull-request reviews and issues, as well as local reviews via a custom
web interface. It renders the content to Markdown designed for LLM consumption.
`repo-to-md` ships embedded skills to allow to easily integrate it into agentic
workflows. `repo-to-md` interacts with GitHub using the `gh` CLI.

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

To format the comments for the current branch's last pull request review, use

```bash
repo-to-md review format
```

This command auto-detects the repository from git remote, if configured. To
format the last review for a specific PR, pass the PR number as the positional
argument:

```bash
repo-to-md review format <PR_NUMBER>
```

Use `--review` to select a specific review ID or index.

To specify owner and repository explicitly, use:

```bash
repo-to-md review format --repo chmp/repo-to-md
```

### AI Agent Skills

This tool includes skills that enable natural language interaction with GitHub
reviews and issues. The skills follow the
[AgentSkills specification](https://agentskills.io/) and can be used by any
compatible AI agent.

**review-to-md** fetches and formats PR review comments. AI agents can use this
skill when addressing review feedback. Example prompts that trigger this skill:

- "Please address my last review on GitHub"
- "Implement the feedback from my PR review"
- "Fix the issues mentioned in the code review"

**issue-to-md** fetches and formats GitHub issues. AI agents can use this skill
when working on issues. Example prompts that trigger this skill:

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

The default behavior auto-detects the PR from the current branch and selects the
last review. Use flags to override:

```bash
repo-to-md review format <PR_NUMBER>                   # Last review on a PR
repo-to-md review format <PR_NUMBER> --review 1        # Review by index
repo-to-md review format <PR_NUMBER> --review <ID>     # Review by ID
repo-to-md review format                               # Last review on current branch's PR
repo-to-md review format --author @me                  # Filter by author
repo-to-md review format --repo owner/repo             # Override repository
repo-to-md review format --local                      # Most recently modified local session
```

For all available options, run `repo-to-md review format --help`.
The `format` subcommand may be omitted when the first argument is not a known
review subcommand, so `repo-to-md review 42` is accepted as shorthand for
`repo-to-md review format 42`.

### Fetching issues

Fetch and format a GitHub issue:

```bash
repo-to-md issue format 42
repo-to-md issue format 42 --repo owner/repo
```

The `format` subcommand may be omitted when the first argument is not a known
issue subcommand, so `repo-to-md issue 42` is accepted as shorthand for
`repo-to-md issue format 42`.

The repository is auto-detected from the `origin` remote when not specified.

### Local review

Review local commits in a web UI before merging to the base branch:

```bash
repo-to-md review local                    # Auto-detect base, review commits up to HEAD
repo-to-md review local main               # Review commits from main to HEAD
repo-to-md review local main feature       # Review commits from main to feature
repo-to-md review local --diff saved.patch  # Review a saved unified diff
repo-to-md review local --commit HEAD~2     # Review one commit against its first parent
repo-to-md review local --from .review-comments/old-session.json
```

This launches a local web server with a side-by-side diff viewer where you can
add comments to the changes. With positional refs, the command reviews a range
of commits (base..end). If the end ref defaults to `HEAD` and there are
uncommitted changes, it prints a warning and continues; working-tree changes
are not included in the review. Range sessions are persisted under
`.review-comments/`, with one JSON file per base and end commit SHA. Reopening
the same commit range resumes its session; a different range gets a separate
session. The directory contains a `.gitignore` so generated session files stay
out of Git. Use `-o` to save to a specific JSON path instead. If that path
already contains a session for a different range, the command reports an error
to protect its comments. Choose a different output path or remove the existing
session file to start fresh.

Use `--diff <path>` to review a saved unified diff file. The source path appears
in the UI. By default, its session is saved as
`.review-comments/diff-<blob-id>.json`, keyed by the exact file contents, so the
same diff resumes its comments even if it is opened through another path. Use
`-o` to choose a session path; an existing session at that path is protected if
the diff contents change.

Use `--commit <ref>` to review one commit against its first parent. Root commits
are compared with the empty tree, and merge commits are compared with their
first parent. Its default session uses the same base and commit SHA naming as a
commit range. `--diff` and `--commit` cannot be combined with each other or with
positional refs. Server options such as `--bind`, `--port`, and `--no-open` work
in either mode.

Use `--from <path>` to continue from an earlier local review JSON file. It uses
the saved diff snapshot when present, otherwise regenerates the diff from the
stored commit SHAs if Git still has those objects, and carries forward comments
and viewed-file progress.
`--from` supplies the diff, so it cannot be combined with refs, `--diff`, or
`--commit`. A separate session is created by default; use `-o` to choose its
path. Repeating the command resumes the generated continuation session.

The bind address and port default to `127.0.0.1` and `8080`. They can be set
with `REPO_TO_MD_BIND` and `REPO_TO_MD_PORT`; explicit `--bind` and `--port`
arguments take precedence over the environment.

For all available options, run `repo-to-md review local --help`.

The most recently modified local session can be exported to markdown by running:

```bash
repo-to-md review format --local
```

You can also pass a local comments file explicitly, including a path supplied
with `review local -o`:

```bash
repo-to-md review format .review-comments/<base-sha>-<end-sha>.json
repo-to-md review format path/to/comments.json
```

When the positional argument names an existing path, `review format` treats it
as a local comments file unless `--remote` is set. Otherwise it is treated as a
PR number. Use `--review` to choose a specific review by ID or index. With no
arguments, `review format` keeps its remote review behavior.

## How it works

This tool uses the GitHub cli `gh` and the Git cli `git` for the following
operations

- `git`: read the configured remotes and the current branch
- `gh`: perform GraphQL queries using `gh api graphql`
