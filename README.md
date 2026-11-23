# coverguard

A CLI that enforces coverage thresholds on the coverage report your tests already write. It reads the report from a stack's own test command, such as llvm-cov export JSON from `swift test --enable-code-coverage` or [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov), or a Go cover profile, and fails when per-file, per-directory, or total coverage falls below the configured thresholds. It checks reports and never runs your tests or produces coverage itself.

This project is at an early stage: it only reads llvm-cov export JSON reports so far, such as the one SwiftPM writes, and takes its thresholds as arguments.

## Usage

Run the tests with coverage, then check the report at the path SwiftPM prints:

```sh
swift test --enable-code-coverage
coverguard check "$(swift test --show-codecov-path)" \
  --file-threshold 100 \
  --exclude '.build/**' \
  --exclude 'Tests/**'
```

Thresholds are percentages, given with `--file-threshold`, `--directory-threshold` (over the files directly in each directory), and `--total-threshold`. They apply to line coverage unless `--metric` picks `functions` or `regions`. Exclude patterns are globs matched against paths relative to the working directory, in which `*` doesn't match across `/`.

coverguard prints each checked file's coverage and anything below its threshold, and exits non-zero when a threshold fails or no file is left to check.

## Setup

Install [rustup](https://rustup.rs/), then install the toolchain pinned in `rust-toolchain.toml`:

```sh
rustup install
```

Install [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) to run the test suite with coverage:

```sh
cargo install cargo-tarpaulin
```

Install [Lefthook](https://lefthook.dev/) and [dprint](https://dprint.dev/), then register the pre-commit hook:

```sh
lefthook install
```

## Development

Run the tests with coverage through tarpaulin:

```sh
cargo tarpaulin
```

The project enforces 100% code coverage, excluding files listed under `exclude-files` in `tarpaulin.toml`.

Each `git commit` runs the pre-commit hook registered during setup, which checks your changes and fixes what it can in place. If it fails, fix any reported issues, re-stage the changed files, and commit again.

After committing, push to `main` or open a pull request from another branch — CI will run the same checks across all files, plus additional checks of its own.

## Releasing

Update the version in `Cargo.toml`, push a version tag, and create a GitHub Release. To publish to [crates.io](https://crates.io/), run:

```sh
cargo publish
```
