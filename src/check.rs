//! Checking file coverage against thresholds.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::report::{Coverage, FileCoverage};

/// The minimum coverage percentages to check, each skipped when absent.
#[derive(Clone, Copy, Debug, Default)]
pub struct Thresholds {
    /// The minimum coverage of each file.
    pub file: Option<f64>,
    /// The minimum coverage of each directory, over the files directly in it.
    pub directory: Option<f64>,
    /// The minimum coverage of all files together.
    pub total: Option<f64>,
}

/// What a threshold was checked against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Subject {
    /// A single file.
    File(PathBuf),
    /// The files directly in a directory.
    Directory(PathBuf),
    /// All files together.
    Total,
}

/// A coverage that falls below its threshold.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    /// What the threshold was checked against.
    pub subject: Subject,
    /// The coverage percentage that was found.
    pub percent: f64,
    /// The threshold it falls below.
    pub threshold: f64,
}

/// The result of checking file coverage against thresholds.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    /// The checked files, with excluded files left out.
    pub files: Vec<FileCoverage>,
    /// The coverage of all checked files together.
    pub total: Coverage,
    /// The coverages that fall below their thresholds.
    pub failures: Vec<Failure>,
}

impl Outcome {
    /// Returns whether any file was checked and every threshold was met.
    #[must_use]
    pub const fn passed(&self) -> bool {
        !self.files.is_empty() && self.failures.is_empty()
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.files.is_empty() {
            return writeln!(f, "No files to check");
        }

        for file in &self.files {
            writeln!(
                f,
                "{}: {:.2}%",
                file.path.display(),
                file.coverage.percent()
            )?;
        }
        writeln!(f, "Total: {:.2}%", self.total.percent())?;

        if !self.failures.is_empty() {
            writeln!(f, "\nBelow threshold:")?;
            for failure in &self.failures {
                match &failure.subject {
                    Subject::File(path) => write!(f, "- file {}", path.display())?,
                    Subject::Directory(path) => {
                        write!(f, "- directory {}", display_directory(path))?;
                    }
                    Subject::Total => write!(f, "- total")?,
                }
                writeln!(f, ": {:.2}% < {:.2}%", failure.percent, failure.threshold)?;
            }
        }
        Ok(())
    }
}

fn display_directory(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.display().to_string()
    }
}

/// Builds a matcher for files to exclude from glob patterns, in which `*`
/// doesn't match across `/`.
///
/// # Errors
///
/// Returns an error if any pattern isn't a valid glob.
pub fn build_excludes(patterns: &[String]) -> Result<GlobSet, globset::Error> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(GlobBuilder::new(pattern).literal_separator(true).build()?);
    }
    builder.build()
}

/// Checks the coverage of the files not matched by `excludes` against the
/// thresholds.
#[must_use]
pub fn check(files: Vec<FileCoverage>, excludes: &GlobSet, thresholds: &Thresholds) -> Outcome {
    let files: Vec<FileCoverage> = files
        .into_iter()
        .filter(|file| !excludes.is_match(&file.path))
        .collect();

    let mut directories = BTreeMap::<PathBuf, Coverage>::new();
    let mut total = Coverage::default();
    for file in &files {
        let directory = file.path.parent().map_or_default(Path::to_path_buf);
        *directories.entry(directory).or_default() += file.coverage;
        total += file.coverage;
    }

    let mut failures = Vec::new();
    if let Some(threshold) = thresholds.file {
        for file in &files {
            push_failure(
                &mut failures,
                Subject::File(file.path.clone()),
                file.coverage,
                threshold,
            );
        }
    }
    if let Some(threshold) = thresholds.directory {
        for (path, coverage) in directories {
            push_failure(&mut failures, Subject::Directory(path), coverage, threshold);
        }
    }
    if let Some(threshold) = thresholds.total {
        push_failure(&mut failures, Subject::Total, total, threshold);
    }

    Outcome {
        files,
        total,
        failures,
    }
}

fn push_failure(failures: &mut Vec<Failure>, subject: Subject, coverage: Coverage, threshold: f64) {
    let percent = coverage.percent();
    if percent < threshold {
        failures.push(Failure {
            subject,
            percent,
            threshold,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, covered: f64, count: f64) -> FileCoverage {
        FileCoverage {
            path: PathBuf::from(path),
            coverage: Coverage { covered, count },
        }
    }

    fn files() -> Vec<FileCoverage> {
        vec![
            file("Package.swift", 1.0, 1.0),
            file("Sources/Lib/A.swift", 10.0, 10.0),
            file("Sources/Lib/B.swift", 6.0, 10.0),
            file("Sources/Lib/Nested/C.swift", 4.0, 4.0),
            file("Tests/LibTests/ATests.swift", 0.0, 5.0),
        ]
    }

    fn excludes(patterns: &[&str]) -> GlobSet {
        let patterns: Vec<String> = patterns.iter().map(ToString::to_string).collect();
        build_excludes(&patterns).unwrap()
    }

    #[test]
    fn pass_without_thresholds() {
        let outcome = check(files(), &GlobSet::empty(), &Thresholds::default());
        assert!(outcome.passed());
        assert_eq!(outcome.files, files());
        assert_eq!(
            outcome.total,
            Coverage {
                covered: 21.0,
                count: 30.0
            }
        );
        assert_eq!(
            outcome.to_string(),
            "Package.swift: 100.00%\n\
             Sources/Lib/A.swift: 100.00%\n\
             Sources/Lib/B.swift: 60.00%\n\
             Sources/Lib/Nested/C.swift: 100.00%\n\
             Tests/LibTests/ATests.swift: 0.00%\n\
             Total: 70.00%\n",
        );
    }

    #[test]
    fn exclude_matching_files() {
        let outcome = check(
            files(),
            &excludes(&["Tests/**", "Sources/*/B.swift"]),
            &Thresholds {
                file: Some(100.0),
                ..Thresholds::default()
            },
        );
        assert!(outcome.passed());
        assert_eq!(
            outcome.files,
            vec![
                file("Package.swift", 1.0, 1.0),
                file("Sources/Lib/A.swift", 10.0, 10.0),
                file("Sources/Lib/Nested/C.swift", 4.0, 4.0),
            ],
        );
    }

    #[test]
    fn keep_star_within_a_directory() {
        let outcome = check(
            files(),
            &excludes(&["Sources/*.swift"]),
            &Thresholds::default(),
        );
        assert_eq!(outcome.files, files());
    }

    #[test]
    fn fail_below_thresholds() {
        let outcome = check(
            files(),
            &excludes(&["Tests/**"]),
            &Thresholds {
                file: Some(80.0),
                directory: Some(90.0),
                total: Some(90.0),
            },
        );
        assert!(!outcome.passed());
        assert_eq!(
            outcome.failures,
            vec![
                Failure {
                    subject: Subject::File(PathBuf::from("Sources/Lib/B.swift")),
                    percent: 60.0,
                    threshold: 80.0,
                },
                Failure {
                    subject: Subject::Directory(PathBuf::from("Sources/Lib")),
                    percent: 80.0,
                    threshold: 90.0,
                },
                Failure {
                    subject: Subject::Total,
                    percent: 84.0,
                    threshold: 90.0,
                },
            ],
        );
        assert_eq!(
            outcome.to_string(),
            "Package.swift: 100.00%\n\
             Sources/Lib/A.swift: 100.00%\n\
             Sources/Lib/B.swift: 60.00%\n\
             Sources/Lib/Nested/C.swift: 100.00%\n\
             Total: 84.00%\n\
             \n\
             Below threshold:\n\
             - file Sources/Lib/B.swift: 60.00% < 80.00%\n\
             - directory Sources/Lib: 80.00% < 90.00%\n\
             - total: 84.00% < 90.00%\n",
        );
    }

    #[test]
    fn display_root_directory() {
        let outcome = check(
            vec![file("main.swift", 1.0, 2.0)],
            &GlobSet::empty(),
            &Thresholds {
                directory: Some(100.0),
                ..Thresholds::default()
            },
        );
        assert_eq!(
            outcome.to_string(),
            "main.swift: 50.00%\n\
             Total: 50.00%\n\
             \n\
             Below threshold:\n\
             - directory .: 50.00% < 100.00%\n",
        );
    }

    #[test]
    fn fail_without_files() {
        let outcome = check(files(), &excludes(&["**"]), &Thresholds::default());
        assert!(!outcome.passed());
        assert_eq!(outcome.to_string(), "No files to check\n");
    }

    #[test]
    fn reject_invalid_glob() {
        assert!(build_excludes(&["Sources/[".to_string()]).is_err());
    }
}
