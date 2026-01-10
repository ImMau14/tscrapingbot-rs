# Contributing to TScrapingBot

Thank you for your interest in contributing. This document explains how to report issues, propose changes, run the project locally, and prepare pull requests.

## Code of Conduct

We expect contributors to follow a respectful, inclusive code of conduct.  
The repository adopts the **Contributor Covenant** Code of Conduct, version 2.1. Please read and follow it before contributing.  
(See: https://www.contributor-covenant.org).

## Ways to contribute

1. **Report bugs**: open an issue with a clear title, steps to reproduce, expected vs actual behavior, and relevant logs or error messages.
2. **Suggest features**: open an issue with a short motivation and an example use case.
3. **Fix bugs / add features**: send a pull request (PR) following the workflow below.

## Workflow (fork → branch → PR)

- Fork the repository.
- Create a feature branch named with the pattern `feature/<short-desc>` or `fix/<short-desc>` or `chore/<short-desc>`.
- Make changes on the branch. Keep changes focused and small.
- Rebase or merge latest `main` before opening a PR.
- Open a PR against `main` with a descriptive title and a clear description of the change.

### Example commands

```bash
git clone https://github.com/ImMau14/tscrapingbot-rs.git
git checkout -b feature/add-foo
# make changes
git add .
git commit -m "Feat: Add ask commands"
git push origin feature/add-foo
```

## Commit message convention

We use **Conventional Commits** for predictable history and automated changelogs. The format is:
`<type>: <short description>`
Common types: `Feat`, `Fix`, `Chore`, `Docs`, `Refactor`, `Test`.
Examples: `Feat: Add ask commands` or `Fix: Handle SSL error`.

## Pull request checklist

Before marking a PR ready for review, ensure:

- CI checks pass.
- `cargo fmt` / `cargo clippy` run locally and fix reported issues.
- Tests added or updated when appropriate (`cargo test`).
- PR description references related issue(s) and describes testing steps.
- Small atomic changes (split large work into multiple PRs).

## Code style & tools

- Format code with `rustfmt` (run `cargo fmt`). We expect a consistent style across the repo.
- Run lints with `clippy` (run `cargo clippy`). Fix warnings as appropriate.

## Tests & CI

- Run the test suite locally with `cargo test`.
- All PRs must pass CI checks (unit tests, clippy, fmt). The repository uses GitHub Actions for CI — required status checks are enforced so PRs cannot be merged until checks succeed.

## Branch protection and merging

- `main` is a protected branch. Merges into `main` require:
  - At least one approving review (configurable).
  - Passing status checks (CI).

- Preferred merge method: **squash-and-merge** for large commits, **rebase** for smaller ones.

## Local development

- See project README for build and deployment instructions.
- Environment variables are required for running the bot locally (use `.env.template` as a base).
- To run locally:

  ```bash
  cargo build --release
  # or
  cargo run --release
  ```

- If your change needs DB migrations, include the migration files and instructions in the PR.

## Security issues

- For security vulnerabilities, please contact maintainers privately (e.g., email) instead of opening a public issue. Provide reproducible steps and impact.

## Adding a new contributor

- New contributors are welcome. Small, well-scoped PRs are the easiest to review.
- Maintainers may ask for changes before merging. Please address review comments promptly.

## Templates and automation

- We recommend adding issue and PR templates in `.github/ISSUE_TEMPLATE/` and `.github/PULL_REQUEST_TEMPLATE.md`.
- Consider automations (dependabot, auto-merge for minor fixes, etc.) to reduce maintenance burden.

## Thank you

Thank you for helping improve TScrapingBot — your contributions matter.
