#![cfg(any(test, feature = "test-support"))]
#![allow(clippy::unwrap_used)]

use std::fs::read_dir;
use std::path::PathBuf;

/// Directory of linguist sample files used for testing and validation.
pub(crate) fn samples_dir() -> PathBuf {
    let pwd = std::env::current_dir().unwrap();
    pwd.join("test/samples")
}

pub fn samples() -> Vec<Sample> {
    let mut samples = Vec::new();
    for entry in read_dir(samples_dir()).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir()
            && let Some(language) = path.file_name().and_then(|x| x.to_str())
        {
            for entry in read_dir(&path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }
                let path = path.as_path();
                let parts = path
                    .components()
                    .rev()
                    .take(2)
                    .map(|x| x.as_os_str().to_str().unwrap())
                    .collect::<Vec<_>>();
                if SKIPS_NOT_SUPPORTED.contains(format!("{}/{}", parts[1], parts[0]).as_str()) {
                    continue;
                }
                samples.push(Sample {
                    language: language.to_string(),
                    path: path.to_path_buf(),
                });
            }
        }
    }
    samples
}

#[cfg(test)]
/// Directory of linguist test fixtures.
pub(crate) fn fixtures_dir() -> PathBuf {
    let pwd = std::env::current_dir().unwrap();
    pwd.join("test/fixtures")
}

pub struct Sample {
    pub language: String,
    pub path: PathBuf,
}

#[cfg(test)]
pub(crate) fn run<F>(name: &'static str, data: &'static str, f: F)
where
    F: Fn(&str, &str) -> bool,
{
    use std::fs::read;

    let samples_dir = samples_dir();
    let fixtures_dir = fixtures_dir();
    for line in data.lines() {
        let parts = line.split(' ').collect::<Vec<_>>();
        if parts.len() < 2 || parts[0].starts_with('#') {
            continue;
        }
        let expected = parts[0] == "Y";
        let path = parts[1..].join(" ");
        let full_path = samples_dir.join(&path);
        let content = if full_path.exists() && full_path.is_file() {
            eprint!(
                "samples/{} should {}be {name} ",
                path,
                if expected { "" } else { "NOT " }
            );
            read(full_path).unwrap()
        } else {
            let full_path = fixtures_dir.join(&path);
            if full_path.exists() && full_path.is_file() {
                eprint!(
                    "fixtures/{} should {}be {name} ",
                    path,
                    if expected { "" } else { "NOT " }
                );
                read(full_path).unwrap()
            } else {
                eprint!(
                    "[path only test]/{} should {}be {name} ",
                    path,
                    if expected { "" } else { "NOT " }
                );
                vec![]
            }
        };
        let content = std::str::from_utf8(content.as_slice()).unwrap();
        assert_eq!(expected, f(path.as_str(), content));
        eprintln!("✔");
    }
}

skips_from_path!(SKIPS_NOT_SUPPORTED, "skips_not_supported.txt");
#[cfg(test)]
skips_from_path!(SKIPS_WRONG_LANGUAGE, "skips_wrong_language.txt");

macro_rules! skips_from_path {
    ($x:ident, $path:expr) => {
        pub(crate) static $x: ::std::sync::LazyLock<std::collections::HashSet<&'static str>> =
            ::std::sync::LazyLock::new(|| {
                include_str!(concat!("../test/", $path))
                    .lines()
                    .filter(|line| !line.starts_with('#') && !line.is_empty() && *line != "\n")
                    .collect()
            });
    };
}
pub(crate) use skips_from_path;
