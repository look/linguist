use std::time::Instant;

crate::regex_set_from_path!(VENDORED_PATH_RULES, "vendor.yml");

/// Detects if the path is vendored.
pub fn is_vendored(path: &str) -> bool {
    let start = Instant::now();
    let is_vendored = VENDORED_PATH_RULES.is_match(path);
    metrics::histogram!("linguist.check.is_vendored_duration",
        "is_vendored" => is_vendored.to_string())
    .record(start.elapsed());
    is_vendored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor() {
        crate::fixtures::run(
            "vendored",
            include_str!("../test/vendored.txt"),
            |path, _| is_vendored(path),
        );
    }
}
