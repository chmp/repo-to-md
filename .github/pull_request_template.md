## Summary

<!-- What changed and why? -->

## Verification

<!-- List checks run and note any relevant manual checks. -->
- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets`
- [ ] `cargo test`
- [ ] `nix run .#frontend-test` (if the UI changed)

## Checklist

- [ ] Add or update tests for behavior changes
- [ ] Update documentation and `CHANGES.md` when needed
- [ ] Review the diff for unrelated changes
