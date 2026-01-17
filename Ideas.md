# Future Enhancement Ideas

This document tracks potential enhancements and optimizations for review-to-md
that are not part of the current implementation.

## Current Workflow

1. Run `cargo run -- <PR_NUMBER>` to fetch review comments
2. Read formatted markdown output
3. Manually address each comment
4. Make changes to codebase

## Potential Enhancements

### JSON Output Mode

- Add `--format json` to output structured data
- Enables programmatic processing of review comments
- Could pipe to other tools or scripts
- **Benefit**: Makes it easier to build tools on top of review-to-md

### Direct Application Mode (`--apply`)

- Add `--apply` mode that creates TODOs or code comments in source files
- Insert review comments as TODO comments at the relevant lines
- **Safety requirements**:
  - Check `git status` to ensure all files are committed before applying
  - Reject if there are uncommitted changes (prevents destructive changes)
  - Add `--force` flag to override safety check if needed
- **Implementation considerations**:
  - Requires careful file manipulation to avoid breaking code
  - Should preserve file formatting and indentation
  - Need to map line numbers from diff to actual file
  - Consider using a marker format like `// TODO(@reviewer): comment text`

### Claude Code Skill (`/review-pr`)

A skill that integrates review-to-md directly into the Claude Code workflow.

**Design**:

```
/review-pr <PR_NUMBER>
```

**Workflow**:

1. Run `review-to-md <PR_NUMBER>` to fetch comments
2. Parse the markdown output (or use JSON format)
3. Create a todo list in Claude Code with each review comment
4. For each comment, navigate to the file and suggest fixes
5. Track completion of review items

**Benefits**:

- Streamlines the workflow from "fetch reviews" → "address reviews"
- Integrates with Claude Code's todo system
- Provides context-aware suggestions for each review
- Natural workflow: `/review-pr 1` → Claude creates plan → User approves →
  Claude addresses

**Implementation Notes**:

- Skill would need access to `gh` CLI and the review-to-md binary
- Could use JSON output format for easier parsing
- Would integrate naturally with Claude's file editing capabilities
- Could combine with the `--apply` mode concept above

### GraphQL API Migration

- Migrate from REST API (`gh api`) to GitHub GraphQL API
- Would enable more advanced filtering and querying
- Could fetch additional context (PR description, related issues, etc.)
- More efficient for complex queries
- Drop resolved comments

### Filter options

- Allow to filter comments by user (or to top-level comments)

### Select single review

- Migrate to GraphQL API
- Allow to restrict to specific reviews only. At the moment all comments are
  considered, even for multiple reviews
- Add an option to select the review via the top-level comment

## Note on Rejected Ideas

- ~~Interactive Mode~~ - Rejected in favor of hands-off automation
- ~~Filter Options (--file)~~ - Will be superseded by GraphQL API migration
- ~~MCP Server~~ - Skills are the preferred integration approach
