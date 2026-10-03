# Changes

## #30

- Local review sessions are stored under `.review-comments/`, keyed by commit
  range or diff contents. Reopening a session resumes it.
- `review local` can review a saved unified diff with `--diff`, one commit with
  `--commit`, or continue an earlier session with `--from`. Continuation carries
  comments and viewed-file progress.
- When `review local` defaults its end ref to `HEAD`, it warns about a dirty
  working tree and continues with committed changes only.
- Diff parsing handles binary Git entries, omitted one-line hunk counts, and
  reports parse errors with source line numbers.
- `review format --local` formats the most recently modified local session.
- The local review UI wraps long diff lines, shows parent directories in the
  file list, reports comment navigation progress, and gives clear save/error
  feedback.
