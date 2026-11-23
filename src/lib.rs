//! A library for enforcing coverage thresholds on existing coverage reports.
//!
//! The library isn't implemented yet; [`status`] only stands in for it.

/// Returns a notice that the library isn't implemented yet.
#[must_use]
pub const fn status() -> &'static str {
    "coverguard is not implemented yet"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_status() {
        assert_eq!(status(), "coverguard is not implemented yet");
    }
}
