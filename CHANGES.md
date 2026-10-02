# Changes

## `feature/quality-of-life`

- Local review sessions are keyed by the resolved commit range and can be
  resumed. Changed ranges use separate sessions, and conflicting custom output
  paths are rejected to protect saved comments.
- `review local` can review a saved unified diff with `--diff` or a single
  commit with `--commit`. For reviews ending at `HEAD`, it warns about a dirty
  working tree and reviews committed changes only. Diff parsing handles binary
  Git entries and reports errors with source locations.
- `review local --from <path>` continues from an earlier session, reusing its
  saved diff or regenerating it from stored commit SHAs when Git has those
  objects, and carrying forward comments and viewed-file progress.
- `review format --local` formats the most recently modified local session.
  Existing session files can also be passed directly, while `--remote` forces
  remote PR lookup.
- The local review UI gives immediate feedback for comment and file status
  changes, with transient toasts reserved for errors.
