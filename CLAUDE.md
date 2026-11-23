# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About This Repository

coverguard is a Rust library and CLI that reads the coverage report a stack's own test command already writes, such as llvm-cov export JSON or a Go cover profile, and fails when coverage falls below configured thresholds. It checks reports and never runs tests or produces coverage itself. So far it only reads llvm-cov export JSON reports (`src/report.rs`) and checks them against thresholds and exclusions given as CLI arguments (`src/check.rs`); a config file comes later.

## Gotchas

- `lefthook run pre-commit` skips every job when nothing is staged, even ones that ignore the staged file list — pass `--all-files` to run it outside an actual commit.
- The pre-commit hook fixes files in place, and the run fails if any file changes. Report that failure and leave the fixes for the user to review and re-stage before committing again.
- `cargo test` doesn't check coverage — only `cargo tarpaulin` applies the 100% threshold from `tarpaulin.toml`.
- `.gitignore` ignores every dotfile and dotdir (`.*`) — a new one needs an explicit `!` exception there or it silently won't be tracked.
