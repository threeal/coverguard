//! Reading coverage from llvm-cov export JSON reports.

use std::collections::BTreeMap;
use std::ops::AddAssign;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A coverage metric that llvm-cov reports for each file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    /// Lines of code.
    Lines,
    /// Functions.
    Functions,
    /// Code regions.
    Regions,
}

/// The number of covered items out of all items of a metric.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Coverage {
    /// The number of covered items.
    pub covered: f64,
    /// The number of all items.
    pub count: f64,
}

impl Coverage {
    /// Returns the covered percentage, which is 100 when there's nothing to
    /// cover.
    #[must_use]
    pub const fn percent(self) -> f64 {
        if self.count > 0.0 {
            self.covered * 100.0 / self.count
        } else {
            100.0
        }
    }
}

impl AddAssign for Coverage {
    fn add_assign(&mut self, other: Self) {
        self.covered += other.covered;
        self.count += other.count;
    }
}

/// The coverage of a single source file.
#[derive(Clone, Debug, PartialEq)]
pub struct FileCoverage {
    /// The file's path, relative to the root when the file lies under it.
    pub path: PathBuf,
    /// The file's coverage of the read metric.
    pub coverage: Coverage,
}

#[derive(Deserialize)]
struct Export {
    data: Vec<ExportData>,
}

#[derive(Deserialize)]
struct ExportData {
    files: Vec<ExportFile>,
}

#[derive(Deserialize)]
struct ExportFile {
    filename: PathBuf,
    summary: ExportSummary,
}

#[derive(Deserialize)]
struct ExportSummary {
    lines: ExportCounts,
    functions: ExportCounts,
    regions: ExportCounts,
}

#[derive(Deserialize)]
struct ExportCounts {
    count: f64,
    covered: f64,
}

/// Reads each file's coverage of the given metric from an llvm-cov export
/// JSON report.
///
/// Paths under `root` are made relative to it, and a file listed in several
/// of the report's `data` entries has its coverage summed. The files are
/// returned sorted by path.
///
/// # Errors
///
/// Returns an error if `json` isn't a valid llvm-cov export JSON report.
pub fn read_report(
    json: &str,
    metric: Metric,
    root: &Path,
) -> Result<Vec<FileCoverage>, serde_json::Error> {
    let export: Export = serde_json::from_str(json)?;

    let mut coverages = BTreeMap::<PathBuf, Coverage>::new();
    for file in export.data.into_iter().flat_map(|data| data.files) {
        let counts = match metric {
            Metric::Lines => file.summary.lines,
            Metric::Functions => file.summary.functions,
            Metric::Regions => file.summary.regions,
        };
        let path = file
            .filename
            .strip_prefix(root)
            .unwrap_or(&file.filename)
            .to_path_buf();
        *coverages.entry(path).or_default() += Coverage {
            covered: counts.covered,
            count: counts.count,
        };
    }

    Ok(coverages
        .into_iter()
        .map(|(path, coverage)| FileCoverage { path, coverage })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = r#"{
        "data": [
            {
                "files": [
                    {
                        "filename": "/repo/Sources/B.swift",
                        "summary": {
                            "lines": {"count": 10, "covered": 5, "percent": 50},
                            "functions": {"count": 4, "covered": 3, "percent": 75},
                            "regions": {"count": 8, "covered": 2, "percent": 25}
                        }
                    },
                    {
                        "filename": "/elsewhere/A.swift",
                        "summary": {
                            "lines": {"count": 2, "covered": 2, "percent": 100},
                            "functions": {"count": 1, "covered": 1, "percent": 100},
                            "regions": {"count": 1, "covered": 1, "percent": 100}
                        }
                    }
                ]
            },
            {
                "files": [
                    {
                        "filename": "/repo/Sources/B.swift",
                        "summary": {
                            "lines": {"count": 10, "covered": 10, "percent": 100},
                            "functions": {"count": 4, "covered": 4, "percent": 100},
                            "regions": {"count": 8, "covered": 8, "percent": 100}
                        }
                    }
                ]
            }
        ]
    }"#;

    fn file(path: &str, covered: f64, count: f64) -> FileCoverage {
        FileCoverage {
            path: PathBuf::from(path),
            coverage: Coverage { covered, count },
        }
    }

    #[test]
    fn compute_percent() {
        let coverage = Coverage {
            covered: 3.0,
            count: 4.0,
        };
        assert_eq!(coverage.percent().to_string(), "75");
        assert_eq!(Coverage::default().percent().to_string(), "100");
    }

    #[test]
    fn read_each_metric() {
        let root = Path::new("/repo");
        assert_eq!(
            read_report(REPORT, Metric::Lines, root).unwrap(),
            vec![
                file("/elsewhere/A.swift", 2.0, 2.0),
                file("Sources/B.swift", 15.0, 20.0),
            ],
        );
        assert_eq!(
            read_report(REPORT, Metric::Functions, root).unwrap(),
            vec![
                file("/elsewhere/A.swift", 1.0, 1.0),
                file("Sources/B.swift", 7.0, 8.0),
            ],
        );
        assert_eq!(
            read_report(REPORT, Metric::Regions, root).unwrap(),
            vec![
                file("/elsewhere/A.swift", 1.0, 1.0),
                file("Sources/B.swift", 10.0, 16.0),
            ],
        );
    }

    #[test]
    fn reject_invalid_report() {
        assert!(read_report(r#"{"data": 1}"#, Metric::Lines, Path::new("/")).is_err());
    }
}
