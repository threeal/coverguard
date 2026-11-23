//! A command-line interface for enforcing coverage thresholds on existing
//! coverage reports.
//!
//! The CLI isn't implemented yet; it only prints the library's status.

fn main() {
    clap::Command::new("coverguard")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Enforce coverage thresholds on the reports your tests already write")
        .get_matches();

    println!("{}", coverguard::status());
}
