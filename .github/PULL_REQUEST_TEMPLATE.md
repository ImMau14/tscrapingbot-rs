---
name: Pull request
about: Describe your changes and how to test them
---

## Summary

Short description of the change.

## Related issue

Closes #<issue-number> (if applicable)

## Changes

- Bullet list of main changes.

## How to test

Steps to reproduce and verify the change locally:

1. Build: `cargo build`
2. Run tests: `cargo test`
3. Format/lint: `cargo fmt --all && cargo clippy --all -- -D warnings`

If DB migrations are needed:

- Add migration under `migrations/` and describe steps here.

## Checklist

- [ ] CI passed
- [ ] `cargo fmt` applied
- [ ] `cargo clippy` passed (no warnings)
- [ ] Tests added/updated (if applicable)
