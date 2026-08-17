<!--
Thanks for contributing to Swini! 🚀
Please follow the guidelines below to ensure your PR is processed quickly.
-->

## Title Reminder ⚠️

This repository uses [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) and a "Squash and Merge" workflow to automatically trigger releases! **Your PR title will become the squashed commit message.**

Please format your PR title as follows:

- `feat: <description>` (Adds a new feature, triggers a MINOR version bump)
- `fix: <description>` (Fixes a bug, triggers a PATCH version bump)
- `chore: <description>` (Internal changes, no version bump)
- `docs: <description>` (Documentation changes, no version bump)
- Use `!` for breaking changes (e.g., `feat!: rewrite API`, triggers a MAJOR version bump)

---

## 📝 What does this PR do?

<!-- Describe the purpose of this PR and the changes it introduces. -->

## 🧪 How was this tested?

<!-- Explain how you tested your changes (e.g., added unit tests, manually ran CLI commands). -->

## ✅ Checklist

- [ ] My PR title follows the Conventional Commits format.
- [ ] I have added/updated tests for my changes (if applicable).
- [ ] I have updated the documentation (if applicable).
- [ ] `cargo check` and `cargo test` pass successfully locally.
- [ ] I have run `cargo fmt` to format my code.
