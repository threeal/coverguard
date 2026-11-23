//! A command-line interface for enforcing coverage thresholds on the coverage
//! reports that a stack's own test command writes.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{ArgAction, ArgMatches, Command, arg, value_parser};
use coverguard::check::{Thresholds, build_excludes, check};
use coverguard::report::{Metric, read_report};

fn main() -> ExitCode {
    let matches = Command::new("coverguard")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Enforce coverage thresholds on the reports your tests already write")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("check")
                .about("Check an llvm-cov export JSON report against coverage thresholds")
                .arg(
                    arg!(<report> "Path to the llvm-cov export JSON report")
                        .value_parser(value_parser!(PathBuf)),
                )
                .arg(
                    arg!(--metric <METRIC> "Coverage metric to check")
                        .value_parser(["lines", "functions", "regions"])
                        .default_value("lines"),
                )
                .arg(
                    arg!(--"file-threshold" <PERCENT> "Minimum coverage of each file")
                        .value_parser(value_parser!(f64)),
                )
                .arg(
                    arg!(--"directory-threshold" <PERCENT> "Minimum coverage of the files directly in each directory")
                        .value_parser(value_parser!(f64)),
                )
                .arg(
                    arg!(--"total-threshold" <PERCENT> "Minimum coverage of all files together")
                        .value_parser(value_parser!(f64)),
                )
                .arg(
                    arg!(--exclude <GLOB> "Exclude files whose path matches the glob, relative to the working directory")
                        .action(ArgAction::Append),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("check", args)) => run_check(args),
        _ => unreachable!("clap requires a known subcommand"),
    }
}

fn run_check(args: &ArgMatches) -> ExitCode {
    let report = args
        .get_one::<PathBuf>("report")
        .expect("report is required");
    let json = match std::fs::read_to_string(report) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("error: failed to read {}: {err}", report.display());
            return ExitCode::FAILURE;
        }
    };

    let metric = match args.get_one::<String>("metric").map(String::as_str) {
        Some("functions") => Metric::Functions,
        Some("regions") => Metric::Regions,
        _ => Metric::Lines,
    };
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(err) => {
            eprintln!("error: failed to get the working directory: {err}");
            return ExitCode::FAILURE;
        }
    };
    let files = match read_report(&json, metric, &root) {
        Ok(files) => files,
        Err(err) => {
            eprintln!("error: failed to parse {}: {err}", report.display());
            return ExitCode::FAILURE;
        }
    };

    let patterns: Vec<String> = args
        .get_many::<String>("exclude")
        .unwrap_or_default()
        .cloned()
        .collect();
    let excludes = match build_excludes(&patterns) {
        Ok(excludes) => excludes,
        Err(err) => {
            eprintln!("error: invalid exclude pattern: {err}");
            return ExitCode::FAILURE;
        }
    };

    let thresholds = Thresholds {
        file: args.get_one::<f64>("file-threshold").copied(),
        directory: args.get_one::<f64>("directory-threshold").copied(),
        total: args.get_one::<f64>("total-threshold").copied(),
    };

    let outcome = check(files, &excludes, &thresholds);
    print!("{outcome}");
    if outcome.passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
