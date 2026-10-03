# Changes

## `feature/quality-of-life`

- Local review sessions are stored under `.review-comments/`, keyed by commit
  range or diff contents. Reopening a session resumes it; changed inputs get a
  separate session, and conflicting custom output paths are rejected to
  protect saved comments.
- `review local` can review a saved unified diff with `--diff`, one commit with
  `--commit`, or continue an earlier session with `--from`. Continuation carries
  comments and viewed-file progress, reuses the saved diff when available, and
  otherwise regenerates it from stored commit SHAs when Git has those objects.
- When `review local` defaults its end ref to `HEAD`, it warns about a dirty
  working tree and continues with committed changes only. `--commit` compares
  against the first parent, or the empty tree for root commits.
- Diff parsing handles binary Git entries, omitted one-line hunk counts, and
  reports parse errors with source line numbers.
- `review format --local` formats the most recently modified local session.
  Existing session files can also be passed directly, while `--remote` forces
  remote PR lookup.
- The local review UI wraps long diff lines, shows parent directories in the
  file list, reports comment navigation progress, and gives clear save/error
  feedback. Successful changes update the UI directly; transient toasts are
  reserved for errors.
- Adds a pull request template, updates CLI and local-review skill
  documentation and contributor guidance, and refreshes the Python tooling
  lockfile.
